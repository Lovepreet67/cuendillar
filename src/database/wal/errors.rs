use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum WALError {
    IOError(String),
    CrrouptedMetadataFile(String),
    UnexpectedEndOfFile(PathBuf),
}
impl From<std::io::Error> for WALError {
    fn from(value: std::io::Error) -> Self {
        WALError::IOError(value.to_string())
    }
}
impl From<uuid::Error> for WALError {
    fn from(value: uuid::Error) -> WALError {
        WALError::CrrouptedMetadataFile(value.to_string())
    }
}
