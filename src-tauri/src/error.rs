use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("baza zablokowana — wymagane odblokowanie hasłem")]
    VaultLocked,

    #[error("nie zaimplementowano: {0}")]
    NotImplemented(&'static str),

    #[error("błąd I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("błąd bazy danych: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("błąd HTTP: {0}")]
    Http(#[from] reqwest::Error),

    #[error("błąd keyring: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("błąd: {0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Other(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
