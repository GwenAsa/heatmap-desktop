use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Validation(String),
    #[error("{0} not found")]
    NotFound(String),
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Invalid data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("File error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Validation(msg.into()))
}

/// Errors cross the IPC boundary as plain, user-readable strings.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
