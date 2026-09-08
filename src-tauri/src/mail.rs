use crate::error::{AppError, AppResult};
use crate::models::{Draft, Settings};
use lettre::message::{header, Mailbox};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

pub const LKML: &str = "linux-kernel@vger.kernel.org";

pub struct Sent {
    pub message_id: String,
}

pub fn new_message_id() -> String {
    let host = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "lkml-pin.local".to_string());
    format!("<{}@{}>", uuid::Uuid::new_v4(), host)
}

pub fn lore_url(message_id: &str) -> String {
    let bare = message_id.trim_matches(|c| c == '<' || c == '>');
    format!("https://lore.kernel.org/all/{bare}/")
}

pub fn parse_address_list(raw: &str) -> AppResult<Vec<Mailbox>> {
    raw.split([',', ';', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<Mailbox>().map_err(AppError::from))
        .collect()
}

pub async fn send(settings: &Settings, password: &str, draft: &Draft) -> AppResult<Sent> {
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
    let message_id = new_message_id();

    let mut builder = Message::builder()
        .from(from)
        .subject(draft.subject.trim())
        .message_id(Some(message_id.clone()))
        .header(header::ContentType::TEXT_PLAIN)
        .header(header::ContentTransferEncoding::EightBit);
    for m in to {
        builder = builder.to(m);
    }
    for m in cc {
        builder = builder.cc(m);
    }
    let email = builder.body(draft.body.clone())?;

    let mut transport = match settings.smtp_security.as_str() {
        "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&settings.smtp_host)?,
        "none" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.smtp_host),
        _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.smtp_host)?,
    }
    .port(settings.smtp_port);
    if !settings.smtp_user.is_empty() {
        transport = transport.credentials(Credentials::new(
            settings.smtp_user.clone(),
            password.to_string(),
        ));
    }
    transport.build().send(email).await?;
    Ok(Sent { message_id })
}
