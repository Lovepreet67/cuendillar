use crate::database::{
    Entry,
    memtable::{Memtable, errors::MemtableError},
};
pub mod default_manager;

pub trait MemtableManager {
    fn insert(&mut self, e: Entry<'_>) -> Result<(), MemtableError>;
    fn find(&self, key: &[u8]) -> Result<Option<Entry<'_>>, MemtableError>;
    fn rotate(&mut self, id: uuid::Uuid) -> Result<(), MemtableError>;
    fn iter(&self) -> impl std::iter::Iterator<Item = Entry>;
}
