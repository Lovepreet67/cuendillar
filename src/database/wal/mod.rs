use crate::database::wal::errors::WALError;

pub mod default_wal;
pub mod errors;
#[cfg(test)]
mod tests;

pub const MAGIC_NUMBER: u64 = 0x123232;

pub trait WAL: Send + Sync {
    /// this function will return the offset to which next log will start
    fn append_log(&mut self, payload: &[u8]) -> Result<u64, WALError>;
    fn read(&self, offset: u64) -> Result<Box<dyn WALIterator>, WALError>;
    /// this function will flush all the logs preceding but not including the offset passed
    fn flush_wal(&mut self, offset: u64) -> Result<(), WALError>;
    /// this function return the offset from which next entry will be assigned
    fn get_offset(&self) -> u64;
}

pub trait WALIterator: Iterator<Item = Result<(u64, Vec<u8>), WALError>> {}
