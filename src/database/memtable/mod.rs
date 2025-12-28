use uuid::Uuid;

use crate::database::memtable::errors::MemtableError;

pub mod errors;
#[cfg(test)]
mod tests;
pub mod vector_memtable;

pub(crate) trait Entry: 'static + Sized {
    fn get_key(&self) -> &[u8];
    fn mark_deleted(&mut self);
    fn is_deleted(&self) -> bool;
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error>;
    fn decode<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error>;
}

pub(crate) trait Memtable<K>
where
    K: Entry,
{
    fn get_id(&self) -> &Uuid;
    fn insert(&mut self, e: K);
    fn delete(&mut self, e: K);
    fn find(&self, key: &[u8]) -> Result<&K, MemtableError>;
    fn iter(&self) -> impl std::iter::Iterator<Item = &K>;
}
