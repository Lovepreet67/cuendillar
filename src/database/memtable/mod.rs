use crate::database::memtable::errors::MemtableError;

mod errors;
mod vector_memetable;

pub(crate) trait Entry {
    fn get_key(&self) -> &[u8];
    fn mark_deleted(&mut self);
    fn is_deleted(&self) -> bool;
}

pub(crate) trait Memtable<K>
where
    K: Entry,
{
    fn insert(&mut self, e: K);
    fn delete(&mut self, e: K);
    fn find(&self, key: &[u8]) -> Result<&K, MemtableError>;
}
