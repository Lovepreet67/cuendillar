pub enum WALWriteError {
    IOError(std::io::Error),
}
impl From<std::io::Error> for WALWriteError {
    fn from(value: std::io::Error) -> Self {
        WALWriteError::IOError(value)
    }
}
pub enum WALReaderError {
    IOError(std::io::Error),
}
impl From<std::io::Error> for WALReaderError {
    fn from(value: std::io::Error) -> Self {
        WALReaderError::IOError(value)
    }
}
