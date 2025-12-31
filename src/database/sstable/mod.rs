use crate::database::{common::Entry, memtable::Memtable};
mod default_sstable;
#[cfg(test)]
mod tests;

pub trait SSTable {
    fn push_memtable<K: Entry, T: Memtable<K>>(&mut self, mt: &T) -> Result<(), std::io::Error>;
    fn build_memtable<K: Entry, T: Memtable<K>>(
        &mut self,
        id: &uuid::Uuid,
    ) -> Result<Vec<K>, std::io::Error>;
}
