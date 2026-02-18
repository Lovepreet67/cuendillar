use uuid::Uuid;

use crate::database::{Entry, memtable::errors::MemtableError};

pub mod errors;
pub mod manager;
#[cfg(test)]
mod tests;
pub mod vector_memtable;

pub trait Memtable: Send + Sync {
    fn get_id(&self) -> &Uuid;
    fn insert(&mut self, e: Entry, wal_offset: u64);
    fn find(&self, key: &[u8]) -> Result<Option<Entry<'_>>, MemtableError>;
    fn iter(&self) -> Box<dyn MemtableIterator<Item = Entry<'_>> + '_>;
    fn size(&self) -> u64;
    fn num_enteries(&self) -> u64;
    fn get_wal_offset(&self) -> u64;
}

pub trait MemtableIterator: Iterator {
    fn get_first_entry(&self) -> Option<Entry<'_>>;
    fn get_last_entry(&self) -> Option<Entry<'_>>;
}
