use crate::database::wal::{errors::WALReaderError, wal_entry::WALEntry};

mod default_wal_reader;
pub trait WALReader {
    fn read(&mut self) -> Result<WALEntry, WALReaderError>;
}
