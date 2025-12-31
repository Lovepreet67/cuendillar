use crate::database::wal::{errors::WALWriteError, wal_entry::WALEntry};

mod default_wal_writer;
pub trait WALWriter {
    fn append(&mut self, entry: WALEntry) -> Result<(), WALWriteError>;
}
