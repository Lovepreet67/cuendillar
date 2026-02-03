use crate::database::{Entry, OwnedEntry, wal::errors::WALError};

pub mod default_wal;
pub mod errors;
#[cfg(test)]
mod tests;
pub mod wal_entry;

pub const MAGIC_NUMBER: u64 = 0x123232;
pub const MAX_PAYLOAD_LEN: u64 = 10000000;

pub trait WAL {
    fn append_log(&mut self, payload: &[u8]) -> Result<(), WALError>;
    fn read(&mut self, offset: u64) -> Result<Box<dyn WALIterator>, WALError>;
    fn flush_wal(&mut self, offset: u64) -> Result<(), WALError>;
}

pub trait WALIterator: Iterator<Item = Result<Vec<u8>, WALError>> {}
