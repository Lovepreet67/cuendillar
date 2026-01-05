#[derive(Debug)]
pub enum WALError {
    IOError(std::io::Error),
    CrrouptedMetadataFile(String),
}
impl From<std::io::Error> for WALError {
    fn from(value: std::io::Error) -> Self {
        WALError::IOError(value)
    }
}
impl From<uuid::Error> for WALError {
    fn from(value: uuid::Error) -> WALError {
        WALError::CrrouptedMetadataFile(value.to_string())
    }
}
