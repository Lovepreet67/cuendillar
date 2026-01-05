use crate::database::{OwnedEntry, memtable::Memtable};
mod default_sstable;
#[cfg(test)]
mod tests;

pub trait SSTable {
    fn push_memtable(&mut self, mt: &impl Memtable) -> Result<(), std::io::Error>;
    fn build_memtable(&mut self, id: &uuid::Uuid) -> Result<Vec<OwnedEntry>, std::io::Error>;
}
