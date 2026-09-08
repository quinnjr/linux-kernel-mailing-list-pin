//! Fetch a thread from lore.kernel.org's public-inbox as an mbox and parse it.

use crate::error::{AppError, AppResult};
use flate2::read::GzDecoder;
use mailparse::{MailHeaderMap, ParsedMail};
use std::io::Read;

#[derive(Debug, Clone)]
pub struct LoreMessage {
    pub message_id: String,
    pub in_reply_to: Option<String>,
    pub from_name: String,
    pub from_addr: String,
    /// RFC 3339 UTC, or the raw Date header if unparseable.
    pub date: String,
    pub subject: String,
    pub body: String,
}

pub fn thread_mbox_url(message_id: &str) -> String {
    format!("https://lore.kernel.org/all/{}/t.mbox.gz", crate::mail::encoded_id(message_id))
}

pub async fn fetch_thread(client: &reqwest::Client, message_id: &str) -> AppResult<Vec<LoreMessage>> {
    let resp = client.get(thread_mbox_url(message_id)).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::NotOnLore);
    }
    let bytes = resp.error_for_status()?.bytes().await?;
    let mut raw = Vec::new();
    GzDecoder::new(&bytes[..])
        .read_to_end(&mut raw)
        .map_err(|e| AppError::Parse(format!("gzip: {e}")))?;
    Ok(parse_mbox(&raw))
}

/// Split an mboxrd file into messages and parse each one.
pub fn parse_mbox(raw: &[u8]) -> Vec<LoreMessage> {
    split_mbox(raw)
        .into_iter()
        .filter_map(|chunk| mailparse::parse_mail(chunk).ok())
        .filter_map(|m| to_lore_message(&m))
        .collect()
}

fn split_mbox(raw: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut pos = 0;
    while pos < raw.len() {
        let end = raw[pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|i| pos + i + 1)
            .unwrap_or(raw.len());
        let line = &raw[pos..end];
        if line.starts_with(b"From ") {
            if let Some(s) = start {
                out.push(&raw[s..pos]);
            }
            start = Some(end);
        }
        pos = end;
    }
    if let Some(s) = start {
        out.push(&raw[s..]);
    }
    out
}

fn to_lore_message(m: &ParsedMail) -> Option<LoreMessage> {
    let h = &m.headers;
    let message_id = h.get_first_value("Message-ID")?;
    let message_id = message_id.trim().to_string();
    if message_id.is_empty() {
        return None;
    }
    // Gnus and friends append a comment after the id; keep only the first <...>.
    let in_reply_to = h
        .get_first_value("In-Reply-To")
        .and_then(|s| first_msg_id(&s));
    let (from_name, from_addr) = parse_from(&h.get_first_value("From").unwrap_or_default());
    let raw_date = h.get_first_value("Date").unwrap_or_default();
    // Dates are compared as strings everywhere, so an unparseable or missing
    // header falls back to "now" instead of poisoning the sort order.
    let date = Some(raw_date.trim())
        .filter(|d| !d.is_empty())
        .and_then(|d| mailparse::dateparse(d).ok())
        .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339();
    let subject = h.get_first_value("Subject").unwrap_or_default();
    let body = text_body(m).unwrap_or_default();
    Some(LoreMessage { message_id, in_reply_to, from_name, from_addr, date, subject, body })
}

fn first_msg_id(s: &str) -> Option<String> {
    let start = s.find('<')?;
    let end = s[start..].find('>')? + start;
    Some(s[start..=end].to_string())
}

fn parse_from(raw: &str) -> (String, String) {
    match mailparse::addrparse(raw) {
        Ok(list) => {
            for addr in list.iter() {
                if let mailparse::MailAddr::Single(s) = addr {
                    return (s.display_name.clone().unwrap_or_default(), s.addr.clone());
                }
            }
            (String::new(), raw.to_string())
        }
        Err(_) => (String::new(), raw.to_string()),
    }
}

/// First text/plain part, unwrapped and stripped of mboxrd ">From " quoting.
fn text_body(m: &ParsedMail) -> Option<String> {
    if m.subparts.is_empty() {
        if m.ctype.mimetype.eq_ignore_ascii_case("text/plain") || m.ctype.mimetype.is_empty() {
            return m.get_body().ok().map(|b| unquote_mboxrd(&b));
        }
        return None;
    }
    m.subparts.iter().find_map(text_body)
}

fn unquote_mboxrd(body: &str) -> String {
    body.lines()
        .map(|l| {
            let stripped = l.trim_start_matches('>');
            if stripped.starts_with("From ") && l.starts_with('>') {
                &l[1..]
            } else {
                l
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MBOX: &str = "From mboxrd@z Thu Jan  1 00:00:00 1970\n\
From: Alice <alice@example.com>\n\
Subject: [PATCH] foo\n\
Message-ID: <a@example.com>\n\
Date: Mon, 1 Jan 2024 10:00:00 +0000\n\
Content-Type: text/plain\n\
\n\
hello\n\
>From here\n\
\n\
From mboxrd@z Thu Jan  1 00:00:00 1970\n\
From: bob@example.com\n\
Subject: Re: [PATCH] foo\n\
Message-ID: <b@example.com>\n\
In-Reply-To: <a@example.com> (Alice's message of 1 Jan)\n\
Date: Mon, 1 Jan 2024 11:00:00 +0000\n\
Content-Type: text/plain\n\
\n\
looks good\n";

    #[test]
    fn splits_and_parses_two_messages() {
        let msgs = parse_mbox(MBOX.as_bytes());
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].message_id, "<a@example.com>");
        assert_eq!(msgs[0].from_name, "Alice");
        assert_eq!(msgs[0].from_addr, "alice@example.com");
        assert_eq!(msgs[0].body.trim(), "hello\nFrom here");
        assert_eq!(msgs[0].date, "2024-01-01T10:00:00+00:00");
        assert_eq!(msgs[1].in_reply_to.as_deref(), Some("<a@example.com>"));
        assert_eq!(msgs[1].from_addr, "bob@example.com");
        assert_eq!(msgs[1].body.trim(), "looks good");
    }

    /// Hits the network; run with `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn fetches_a_real_lkml_thread() {
        let client = reqwest::Client::builder().user_agent("lkml-pin-test").build().unwrap();
        let msgs = fetch_thread(&client, "<20260828055926.346744-1-vernon2gm@gmail.com>")
            .await
            .unwrap();
        assert!(msgs.len() >= 2, "expected a thread with replies, got {}", msgs.len());
        assert!(msgs.iter().any(|m| m.in_reply_to.is_some()));
        assert!(msgs.iter().all(|m| !m.message_id.is_empty() && !m.date.is_empty()));
        let err = fetch_thread(&client, "<nope-123@nowhere.invalid>").await.unwrap_err();
        assert!(matches!(err, AppError::NotOnLore));
    }

    #[test]
    fn url_strips_brackets() {
        assert_eq!(
            thread_mbox_url("<x@y>"),
            "https://lore.kernel.org/all/x@y/t.mbox.gz"
        );
    }
}
