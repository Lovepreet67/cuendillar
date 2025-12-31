use uuid::Uuid;

use crate::database::{common::Entry, memtable::errors::MemtableError};

pub mod errors;
#[cfg(test)]
mod tests;
pub mod vector_memtable;

pub(crate) trait Memtable<K>
where
    K: Entry,
{
    fn new(id: Option<Uuid>) -> Self;
    fn get_id(&self) -> &Uuid;
    fn insert(&mut self, e: K);
    fn delete(&mut self, e: K);
    fn find(&self, key: &[u8]) -> Result<&K, MemtableError>;
    fn iter(&self) -> impl std::iter::Iterator<Item = &K>;
    fn size(&self) -> u64;
    fn num_enteries(&self) -> u64;
}
