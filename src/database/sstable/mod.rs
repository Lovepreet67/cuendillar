use crate::database::{OwnedEntry, memtable::Memtable, sstable::errors::SSTableError};
pub mod bloom_filter;
pub mod default_sstable;
pub mod errors;
#[cfg(test)]
mod tests;

pub trait SSTable {
    fn push_memtable(&mut self, mt: &impl Memtable) -> Result<(), SSTableError>;
    fn find(&self, id: &[u8]) -> Result<Option<OwnedEntry>, SSTableError>;
}
