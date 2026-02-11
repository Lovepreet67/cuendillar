use std::str::Utf8Error;

use crate::database::wal::errors::WALError;

#[derive(Debug)]
pub enum SSTableError {
    IoError(std::io::Error),
    UuidError(uuid::Error),
    General(String),
    PoisonedError,
}

impl From<WALError> for SSTableError {
    fn from(value: WALError) -> Self {
        return SSTableError::General(format!("{:?}", value));
    }
}

impl From<std::io::Error> for SSTableError {
    fn from(value: std::io::Error) -> Self {
        return SSTableError::IoError(value);
    }
}
impl From<uuid::Error> for SSTableError {
    fn from(value: uuid::Error) -> Self {
        return SSTableError::UuidError(value);
    }
}
impl From<Utf8Error> for SSTableError {
    fn from(value: Utf8Error) -> Self {
        return Self::General(format!(
            "Error while encoding the bytes to utf8, {:?}",
            value
        ));
    }
}
