use std::sync::Arc;

use crate::database::{
    Entry,
    iterator::merged_iterator::MergedIterator,
    memtable::{Memtable, errors::MemtableError},
};
pub mod default_manager;

#[cfg(test)]
mod tests;

pub trait MemtableManager: Send + Sync {
    fn insert(&mut self, e: Entry<'_>, wal_offset: u64) -> Result<(), MemtableError>;
    fn find(&self, key: &[u8]) -> Result<Option<Entry<'_>>, MemtableError>;
    fn rotate(&mut self, id: uuid::Uuid) -> Result<(), MemtableError>;
    fn iter(&self) -> MergedIterator;
    fn require_rotation(&self) -> bool;
    fn get_memtable_to_push(&self) -> Option<Arc<dyn Memtable>>;
    fn mark_pushed(&mut self, memtable_id: uuid::Uuid) -> Result<(), MemtableError>;
}
