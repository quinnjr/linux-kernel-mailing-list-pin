use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("settings incomplete: {0}")]
    Settings(String),
    #[error("keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("invalid address: {0}")]
    Address(#[from] lettre::address::AddressError),
    #[error("could not build message: {0}")]
    Build(#[from] lettre::error::Error),
    #[error("SMTP error: {0}")]
    Smtp(#[from] lettre::transport::smtp::Error),
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("lore.kernel.org has not indexed this message yet")]
    NotOnLore,
    #[error("could not parse mailbox: {0}")]
    Parse(String),
    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
