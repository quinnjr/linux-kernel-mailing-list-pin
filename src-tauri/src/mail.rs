use crate::error::{AppError, AppResult};
use crate::models::{Draft, Settings};
use lettre::message::{header, Mailbox};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::time::Duration;

pub const LKML: &str = "linux-kernel@vger.kernel.org";

/// Whole SMTP session, including the TLS upgrade and every server reply.
/// lettre's own timeout covers only the TCP connect.
const SEND_DEADLINE: Duration = Duration::from_secs(90);
const TEST_DEADLINE: Duration = Duration::from_secs(25);

/// Like git send-email: `<uuid@sender-domain>`, so nothing local leaks into
/// the permanent lore URL.
pub fn new_message_id(sender: &str) -> String {
    let domain = sender
        .rsplit_once('@')
        .map(|(_, d)| d.trim())
        .filter(|d| !d.is_empty())
        .unwrap_or("lkml-pin.invalid");
    format!("<{}@{}>", uuid::Uuid::new_v4(), domain)
}

pub fn bare_id(message_id: &str) -> &str {
    message_id.trim().trim_matches(|c| c == '<' || c == '>')
}

const ID_ESCAPES: &percent_encoding::AsciiSet =
    &NON_ALPHANUMERIC.remove(b'@').remove(b'.').remove(b'-').remove(b'_');

/// Message-ID as a URL path segment.
pub fn encoded_id(message_id: &str) -> String {
    utf8_percent_encode(bare_id(message_id), ID_ESCAPES).to_string()
}

pub fn lore_url(message_id: &str) -> String {
    format!("https://lore.kernel.org/all/{}/", encoded_id(message_id))
}

/// Parse a recipient list the way people actually paste it: RFC 5322
/// mailboxes separated by commas, semicolons, newlines, or (for bare
/// addresses) plain whitespace, with `(maintainer:USB)` comments from
/// get_maintainer.pl removed.
pub fn parse_address_list(raw: &str) -> AppResult<Vec<Mailbox>> {
    split_mailboxes(raw)?
        .into_iter()
        .map(|s| s.parse::<Mailbox>().map_err(AppError::from))
        .collect()
}

/// Tokenise into individual mailbox strings, honouring quotes and angle
/// brackets, and dropping comments and empty entries.
fn split_mailboxes(raw: &str) -> AppResult<Vec<String>> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut in_angle = false;
    let mut comment_depth = 0usize;
    let mut flush = |cur: &mut String| {
        let t = cur.trim();
        if !t.is_empty() {
            out.push(t.to_string());
        }
        cur.clear();
    };
    for c in raw.chars() {
        if comment_depth > 0 {
            match c {
                '(' => comment_depth += 1,
                ')' => comment_depth -= 1,
                _ => {}
            }
            continue;
        }
        if quoted {
            cur.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = false;
            }
            continue;
        }
        match c {
            '"' => {
                quoted = true;
                cur.push(c);
            }
            '(' => comment_depth = 1,
            ')' => return Err(AppError::Settings("unbalanced ')' in recipient list".into())),
            '<' => {
                in_angle = true;
                cur.push(c);
            }
            '>' => {
                in_angle = false;
                cur.push(c);
            }
            ',' | ';' | '\n' | '\r' if !in_angle => flush(&mut cur),
            // Whitespace ends a mailbox once it is complete: after a closing '>'
            // or after a bare address. A display name never contains '@', so
            // this cannot split "Greg KH <...>" before its angle-addr.
            c if c.is_whitespace()
                && !in_angle
                && (cur.ends_with('>') || (cur.contains('@') && !cur.contains('<'))) =>
            {
                flush(&mut cur)
            }
            c => cur.push(c),
        }
    }
    if comment_depth > 0 {
        return Err(AppError::Settings("unclosed '(' in recipient list".into()));
    }
    if quoted {
        return Err(AppError::Settings("unclosed '\"' in recipient list".into()));
    }
    flush(&mut cur);
    Ok(out)
}

fn transport(settings: &Settings, password: &str) -> AppResult<AsyncSmtpTransport<Tokio1Executor>> {
    let mut t = match settings.smtp_security.as_str() {
        "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&settings.smtp_host)?,
        "none" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.smtp_host),
        _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.smtp_host)?,
    }
    .port(settings.smtp_port)
    .timeout(Some(Duration::from_secs(20)));
    if !password.is_empty() && settings.smtp_user.is_empty() {
        return Err(AppError::Settings("a password needs a username".into()));
    }
    if !password.is_empty() && settings.smtp_security == "none" {
        return Err(AppError::Settings(
            "refusing to send a password over an unencrypted connection; choose STARTTLS or TLS".into(),
        ));
    }
    if !settings.smtp_user.is_empty() {
        t = t.credentials(Credentials::new(settings.smtp_user.clone(), password.to_string()));
    }
    Ok(t.build())
}

/// Connect, negotiate TLS and, when a username is set, authenticate. Returns
/// a sentence describing exactly what was proven.
pub async fn test_connection(settings: &Settings, password: &str) -> AppResult<String> {
    if settings.smtp_host.is_empty() {
        return Err(AppError::Settings("enter an SMTP host first".into()));
    }
    let t = transport(settings, password)?;
    let ok = tokio::time::timeout(TEST_DEADLINE, t.test_connection())
        .await
        .map_err(|_| AppError::Other(format!("{} did not answer within {}s", settings.smtp_host, TEST_DEADLINE.as_secs())))?
        .map_err(|e| explain_smtp(e, settings))?;
    if !ok {
        return Err(AppError::Other("server did not answer NOOP".into()));
    }
    Ok(if settings.smtp_user.is_empty() {
        format!("Connected to {}:{}. No login attempted because Username is empty.", settings.smtp_host, settings.smtp_port)
    } else {
        format!("Connected to {}:{} and signed in as {}.", settings.smtp_host, settings.smtp_port, settings.smtp_user)
    })
}

/// Turn lettre's error into the fix, using the error's kind rather than
/// its wording.
fn explain_smtp(e: lettre::transport::smtp::Error, settings: &Settings) -> AppError {
    let google = settings.smtp_host.contains("gmail") || settings.smtp_host.contains("google");
    let code = e.status().map(|s| s.to_string()).unwrap_or_default();
    let text = match code.as_str() {
        "535" | "534" if google => "Google rejected the sign-in. Use a 16-character App Password (2-Step Verification must be on) and your full address as the username.".to_string(),
        "535" | "534" => "The server rejected the username or password.".to_string(),
        "530" => "The server requires a login; enter a username and password.".to_string(),
        _ if e.is_tls() => format!("TLS failed: {e}"),
        _ if e.is_client() || e.is_transient() || e.is_permanent() => format!("SMTP error: {e}"),
        _ => {
            let hint = match settings.smtp_security.as_str() {
                "tls" => " If the server expects STARTTLS (usually port 587), switch Security.",
                "starttls" => " If the server expects implicit TLS (usually port 465), switch Security.",
                _ => "",
            };
            format!("Could not talk to {}:{}: {e}.{hint}", settings.smtp_host, settings.smtp_port)
        }
    };
    AppError::Other(text)
}

/// Build the outgoing message. Kept separate from sending so the Message-ID
/// can be recorded before the irreversible SMTP transaction.
pub fn build(settings: &Settings, draft: &Draft, message_id: &str) -> AppResult<Message> {
    if settings.email.is_empty() || settings.smtp_host.is_empty() {
        return Err(AppError::Settings(
            "fill in your email address and SMTP host in Settings first".into(),
        ));
    }
    let from = Mailbox::new(
        Some(settings.display_name.clone()).filter(|n| !n.is_empty()),
        settings.email.parse()?,
    );
    let to = parse_address_list(&draft.to)?;
    if to.is_empty() {
        return Err(AppError::Settings("at least one To recipient is required".into()));
    }
    let cc = parse_address_list(&draft.cc)?;

    // No explicit Content-Transfer-Encoding: lettre picks 7bit, quoted-printable
    // or base64 from the body; forcing 8bit panics on lines over 74 characters.
    let mut builder = Message::builder()
        .from(from)
        .subject(draft.subject.trim())
        .message_id(Some(message_id.to_string()))
        .header(header::ContentType::TEXT_PLAIN);
    for m in to {
        builder = builder.to(m);
    }
    for m in cc {
        builder = builder.cc(m);
    }
    Ok(builder.body(draft.body.clone())?)
}

pub async fn send(settings: &Settings, password: &str, email: Message) -> AppResult<()> {
    let t = transport(settings, password)?;
    tokio::time::timeout(SEND_DEADLINE, t.send(email))
        .await
        .map_err(|_| AppError::Other(format!("{} stopped responding during the send; it may or may not have accepted the message", settings.smtp_host)))?
        .map_err(|e| explain_smtp(e, settings))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addrs(raw: &str) -> Vec<String> {
        parse_address_list(raw).unwrap().iter().map(|m| m.email.to_string()).collect()
    }

    #[test]
    fn address_list_handles_quotes_and_maintainer_comments() {
        let v = parse_address_list(
            "\"Doe, John\" <j@x.org>, Greg KH <gregkh@lf.org> (supporter:USB)\nlinux-usb@vger.kernel.org (open list:USB)",
        )
        .unwrap();
        let a: Vec<String> = v.iter().map(|m| m.email.to_string()).collect();
        assert_eq!(a, ["j@x.org", "gregkh@lf.org", "linux-usb@vger.kernel.org"]);
        assert_eq!(v[0].name.as_deref(), Some("Doe, John"));
    }

    #[test]
    fn address_list_tolerates_pasted_shapes() {
        assert!(parse_address_list("  \n ").unwrap().is_empty());
        assert_eq!(addrs("a@b.org "), ["a@b.org"]);
        assert_eq!(addrs("a@b.org;"), ["a@b.org"]);
        assert_eq!(addrs("a@b.org c@d.org"), ["a@b.org", "c@d.org"]);
        assert_eq!(addrs("Greg KH <gregkh@lf.org> linux-usb@vger.kernel.org"), ["gregkh@lf.org", "linux-usb@vger.kernel.org"]);
        assert_eq!(addrs("a@b.org (x) c@d.org (open list)"), ["a@b.org", "c@d.org"]);
    }

    #[test]
    fn unbalanced_comment_is_an_error_not_a_dropped_recipient() {
        assert!(parse_address_list("Jane <jane@x.org> (typo, bob@y.org, carol@z.org").is_err());
        assert!(parse_address_list("a@b.org)").is_err());
    }

    #[test]
    fn long_lines_do_not_panic() {
        let s = Settings { email: "a@b.org".into(), smtp_host: "h".into(), ..Settings::defaults() };
        let d = Draft { to: LKML.into(), cc: String::new(), subject: "x".into(), body: "y".repeat(400) };
        build(&s, &d, "<id@b.org>").unwrap();
    }

    #[test]
    fn message_id_uses_sender_domain() {
        assert!(new_message_id("me@example.org").ends_with("@example.org>"));
        assert_eq!(lore_url("<a/b?c@d.org>"), "https://lore.kernel.org/all/a%2Fb%3Fc@d.org/");
    }
}
