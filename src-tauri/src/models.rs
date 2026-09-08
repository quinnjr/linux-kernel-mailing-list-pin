use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub display_name: String,
    pub email: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    /// "starttls" | "tls" | "none"
    pub smtp_security: String,
    pub poll_minutes: u32,
    pub has_password: bool,
}

impl Settings {
    pub fn defaults() -> Self {
        Self {
            smtp_port: 587,
            smtp_security: "starttls".into(),
            poll_minutes: 15,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Draft {
    pub to: String,
    pub cc: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThreadSummary {
    pub id: i64,
    pub message_id: String,
    pub subject: String,
    pub to_addr: String,
    pub cc: String,
    pub sent_at: String,
    pub last_checked_at: Option<String>,
    pub lore_url: String,
    pub reply_count: i64,
    pub unread_count: i64,
    pub last_activity: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Reply {
    pub id: i64,
    pub thread_id: i64,
    pub message_id: String,
    pub in_reply_to: Option<String>,
    pub from_name: String,
    pub from_addr: String,
    pub date: String,
    pub subject: String,
    pub body: String,
    pub read: bool,
    pub lore_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThreadDetail {
    pub summary: ThreadSummary,
    pub body: String,
    pub replies: Vec<Reply>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshReport {
    pub thread_id: i64,
    pub new_replies: usize,
    pub on_lore: bool,
}
