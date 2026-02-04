use uuid::Uuid;

use crate::database::{Entry, memtable::errors::MemtableError};

pub mod errors;
pub mod manager;
#[cfg(test)]
mod tests;
pub mod vector_memtable;

pub trait Memtable {
    fn get_id(&self) -> &Uuid;
    fn insert(&mut self, e: Entry);
    fn find(&self, key: &[u8]) -> Result<Option<Entry>, MemtableError>;
    fn iter(&self) -> Box<dyn MemtableIterator<Item = Entry<'_>> + '_>;
    fn size(&self) -> u64;
    fn num_enteries(&self) -> u64;
}

pub trait MemtableIterator: Iterator {
    fn get_first_entry(&self) -> Option<Entry<'_>>;
    fn get_last_entry(&self) -> Option<Entry<'_>>;
}
