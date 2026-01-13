mod errors;
#[cfg(test)]
mod tests;
#[cfg(test)]
use std::default;
use std::{collections::VecDeque, path::PathBuf, str::FromStr};

use crate::database::{
    Entry, OwnedEntry,
    db_engine::errors::EngineError,
    memtable::{
        Memtable,
        manager::{MemtableManager, default_manager::DefaultManger},
        vector_memtable::VectorMemtable,
    },
    sstable::{SSTable, default_sstable::DefaultSSTable},
    wal::{WAL, default_wal::DefaultWAL, wal_entry::WALEntry},
};

#[derive(Default, Debug)]
pub struct Metrics {
    sstable_hits: u64,
    memtable_hits: u64,
    write_count: u64,
}

pub struct Engine {
    wal_manager: DefaultWAL,
    memtable_manager: DefaultManger<VectorMemtable>,
    sstable: DefaultSSTable,
    write_count: u64,
    pub metrics: Metrics,
}

impl Engine {
    fn push_memetable(&mut self) -> Result<(), EngineError> {
        // we will fetch the immutable table from the memtable_manager
        let ready_to_push_memetable = self.memtable_manager.get_memtable_to_push();
        if ready_to_push_memetable.is_none() {
            return Ok(());
        }
        let ready_to_push_memetable = ready_to_push_memetable.unwrap();
        // push it to sstable.
        self.sstable.push_memtable(ready_to_push_memetable)?;
        // signal memetbale manager to remove that memtable
        self.wal_manager
            .flush_wal(ready_to_push_memetable.get_id().clone())?;
        self.memtable_manager
            .mark_pushed(ready_to_push_memetable.get_id().clone())?;
        return Ok(());
    }
    pub fn new(root_path: &str) -> Result<Self, EngineError> {
        let uid = uuid::Uuid::new_v4();
        let first_memtable = VectorMemtable::new(Some(uid));
        let memetable_manager = DefaultManger::intialize(first_memtable, VecDeque::new(), 500);
        let mut wal_manager = DefaultWAL::new(PathBuf::from_str(root_path)?.join("wal")).unwrap();
        wal_manager.rotate(Some(uid))?;
        let sstable = DefaultSSTable::new(PathBuf::from_str(root_path)?.join("sstable"))?;
        Ok(Self {
            wal_manager: wal_manager,
            memtable_manager: memetable_manager,
            sstable,
            write_count: 0,
            metrics: Metrics::default(),
        })
    }
    pub fn write(&mut self, e: Entry) -> Result<(), EngineError> {
        self.metrics.write_count += 1;
        if self.write_count % 500 == 0 {
            if self.memtable_manager.require_rotation() {
                self.memtable_rotation()?;
            }
            self.push_memetable()?;
        }
        self.wal_manager.append_log(WALEntry::from_entry(&e))?;
        self.memtable_manager.insert(e)?;
        self.write_count += 1;
        Ok(())
    }
    pub fn find(&mut self, key: &[u8]) -> Result<Option<OwnedEntry>, EngineError> {
        self.metrics.memtable_hits += 1;
        let result = self.memtable_manager.find(key)?;
        if result.is_none() {
            self.metrics.sstable_hits += 1;
            let sstable_result = self.sstable.find(key)?;
            return Ok(sstable_result);
        }
        Ok(match result {
            Some(x) => Some(x.into()),
            None => None,
        })
    }
    pub fn memtable_rotation(&mut self) -> Result<(), EngineError> {
        let uid = uuid::Uuid::new_v4();
        self.memtable_manager.rotate(uid)?;
        self.wal_manager.rotate(Some(uid))?;
        Ok(())
    }
}
