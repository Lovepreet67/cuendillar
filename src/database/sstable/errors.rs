#[derive(Debug)]
pub enum SSTableError {
    IoError(std::io::Error),
    UuidError(uuid::Error),
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
