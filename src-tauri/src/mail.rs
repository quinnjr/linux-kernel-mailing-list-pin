use crate::error::{AppError, AppResult};
use crate::models::{Draft, Settings};
use lettre::message::{header, Mailbox, Mailboxes};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

pub const LKML: &str = "linux-kernel@vger.kernel.org";

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

pub fn lore_url(message_id: &str) -> String {
    let id = utf8_percent_encode(bare_id(message_id), ID_ESCAPES);
    format!("https://lore.kernel.org/all/{id}/")
}

/// Parse an RFC 5322 mailbox list. Parenthesised comments such as the
/// `(maintainer:USB)` suffixes from get_maintainer.pl are stripped first,
/// since lettre's grammar does not accept them.
pub fn parse_address_list(raw: &str) -> AppResult<Vec<Mailbox>> {
    let joined = raw.replace(['\n', ';'], ",");
    let cleaned = strip_comments(&joined);
    if cleaned.trim().trim_matches(',').trim().is_empty() {
        return Ok(Vec::new());
    }
    let list: Mailboxes = cleaned.parse()?;
    Ok(list.into_iter().collect())
}

fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0usize;
    let mut quoted = false;
    for c in s.chars() {
        match c {
            '"' if depth == 0 => {
                quoted = !quoted;
                out.push(c);
            }
            '(' if !quoted => depth += 1,
            ')' if !quoted && depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

fn transport(settings: &Settings, password: &str) -> AppResult<AsyncSmtpTransport<Tokio1Executor>> {
    let mut t = match settings.smtp_security.as_str() {
        "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&settings.smtp_host)?,
        "none" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.smtp_host),
        _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.smtp_host)?,
    }
    .port(settings.smtp_port);
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

/// Connect, negotiate TLS and authenticate without sending anything.
pub async fn test_connection(settings: &Settings, password: &str) -> AppResult<()> {
    if settings.smtp_host.is_empty() {
        return Err(AppError::Settings("enter an SMTP host first".into()));
    }
    let ok = transport(settings, password)?.test_connection().await?;
    if ok {
        Ok(())
    } else {
        Err(AppError::Other("server did not answer NOOP".into()))
    }
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

    // No explicit Content-Transfer-Encoding: lettre picks 7bit/8bit/quoted-printable
    // from the body, and forcing 8bit panics on lines over 74 characters.
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
    transport(settings, password)?.send(email).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_list_handles_quotes_and_maintainer_comments() {
        let v = parse_address_list(
            "\"Doe, John\" <j@x.org>, Greg KH <gregkh@lf.org> (supporter:USB)\nlinux-usb@vger.kernel.org",
        )
        .unwrap();
        let addrs: Vec<String> = v.iter().map(|m| m.email.to_string()).collect();
        assert_eq!(addrs, ["j@x.org", "gregkh@lf.org", "linux-usb@vger.kernel.org"]);
        assert_eq!(v[0].name.as_deref(), Some("Doe, John"));
        assert!(parse_address_list("  \n ").unwrap().is_empty());
    }

    #[test]
    fn long_lines_do_not_panic() {
        let s = Settings { email: "a@b.org".into(), smtp_host: "h".into(), ..Settings::defaults() };
        let d = Draft { to: "linux-kernel@vger.kernel.org".into(), cc: String::new(), subject: "x".into(), body: "y".repeat(400) };
        build(&s, &d, "<id@b.org>").unwrap();
    }

    #[test]
    fn message_id_uses_sender_domain() {
        assert!(new_message_id("me@example.org").ends_with("@example.org>"));
        assert_eq!(lore_url("<a/b?c@d.org>"), "https://lore.kernel.org/all/a%2Fb%3Fc@d.org/");
    }
}
