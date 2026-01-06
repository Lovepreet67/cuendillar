use uuid::Uuid;

use crate::database::{Entry, memtable::errors::MemtableError};

pub mod errors;
pub mod manager;
#[cfg(test)]
mod tests;
pub mod vector_memtable;

pub(crate) trait Memtable {
    fn new(id: Option<Uuid>) -> Self;
    fn get_id(&self) -> &Uuid;
    fn insert(&mut self, e: Entry);
    fn find(&self, key: &[u8]) -> Result<Option<Entry>, MemtableError>;
    fn iter(&self) -> impl std::iter::Iterator<Item = Entry>;
    fn size(&self) -> u64;
    fn num_enteries(&self) -> u64;
}
