use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum WALError {
    IOError(String),
    UnexpectedEndOfFile(PathBuf),
    corruptedEntry(u64),
    PayloadLengthOutOfBound(u64),
    InvalidFileName(PathBuf),
}
impl From<std::io::Error> for WALError {
    fn from(value: std::io::Error) -> Self {
        WALError::IOError(value.to_string())
    }
}
