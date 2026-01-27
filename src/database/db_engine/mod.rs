mod errors;
#[cfg(test)]
mod tests;
use std::{
    path::PathBuf,
    str::FromStr,
    sync::{Arc, RwLock},
};

use crate::database::{
    Entry, OwnedEntry,
    config::CONFIG,
    db_engine::errors::EngineError,
    factory::{memtable::build_memtable_manager, wal::build_wal_manger},
    memtable::manager::MemtableManager,
    sstable::{
        cleaner::Cleaner, compaction::leveled_compaction::LevelCompaction,
        version_manager::VersionManager,
    },
    wal::{WAL, wal_entry::WALEntry},
};

#[derive(Default, Debug)]
pub struct Metrics {
    sstable_hits: u64,
    memtable_hits: u64,
    write_count: u64,
}

pub struct Engine {
    wal_manager: Box<dyn WAL>,
    memtable_manager: Box<dyn MemtableManager>,
    version_manager: Arc<RwLock<VersionManager>>,
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
        let sst_meta = self
            .version_manager
            .read()?
            .push_memtable(ready_to_push_memetable)?;
        self.version_manager.write()?.push_l0_update(sst_meta);
        // signal memetbale manager to remove that memtable
        self.wal_manager
            .flush_wal(ready_to_push_memetable.get_id().clone())?;
        self.memtable_manager
            .mark_pushed(ready_to_push_memetable.get_id().clone())?;
        return Ok(());
    }
    pub fn new(root_path: &str) -> Result<Self, EngineError> {
        let uid = uuid::Uuid::new_v4();
        let memetable_manager = build_memtable_manager(&CONFIG.memtable, Some(uid))?;
        let mut wal_manager =
            build_wal_manger(&CONFIG.wal, PathBuf::from_str(root_path)?.join("wal"))?;
        wal_manager.rotate(Some(uid))?;
        let version_manager = Arc::new(RwLock::new(VersionManager::new(
            PathBuf::from_str(root_path)?.join("sstable"),
        )));
        let level_compaction = LevelCompaction::new(
            version_manager.clone(),
            3,
            PathBuf::from_str(root_path)?.join("sstable"),
        );
        level_compaction.init();
        let cleaner = Cleaner::new(version_manager.clone());
        cleaner.init();
        Ok(Self {
            wal_manager: wal_manager,
            memtable_manager: memetable_manager,
            version_manager: version_manager.clone(),
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
            let version_manager = self.version_manager.read()?;
            let latest_version = version_manager.get_latest_version();
            let sstable_result = latest_version.find(key)?;
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
