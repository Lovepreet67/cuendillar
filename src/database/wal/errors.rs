use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum WALError {
    IOError(String),
    UnexpectedEndOfFile(PathBuf),
    CorruptedEntry(u64),
    PayloadLengthOutOfBound(u64),
    InvalidFileName(PathBuf),
    OffsetUnderflow, // this will happen which read request is made for the offset which is flushed i.e first file offset > offset request
    OffsetOverflow,  // this will happen when read request contain offset >= curr offset
}
impl From<std::io::Error> for WALError {
    fn from(value: std::io::Error) -> Self {
        WALError::IOError(value.to_string())
    }
}
