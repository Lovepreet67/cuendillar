mod errors;
#[cfg(test)]
mod tests;
use std::{
    sync::{Arc, RwLock, atomic::AtomicBool},
    thread::{self, JoinHandle, sleep},
    time::Duration,
};

use crate::database::{
    Entry, OwnedEntry,
    config::DbConfig,
    db_engine::errors::EngineError,
    factory::{
        compaction::build_compaction, memtable::build_memtable_manager, wal::build_wal_manger,
    },
    memtable::manager::MemtableManager,
    sstable::{cleaner::Cleaner, version_manager::VersionManager},
    wal::WAL,
};

#[derive(Default, Clone, Copy, Debug)]
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
    workers: Vec<JoinHandle<u64>>,
    under_shutdown: Arc<AtomicBool>,
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
        self.version_manager
            .write()?
            .push_l0_update(sst_meta, ready_to_push_memetable.get_wal_offset());
        // signal memetbale manager to remove that memtable
        self.wal_manager
            .flush_wal(ready_to_push_memetable.get_wal_offset())?; // avoiding flush foor now
        self.memtable_manager
            .mark_pushed(ready_to_push_memetable.get_id().clone())?;
        return Ok(());
    }
    pub fn new(config: Arc<DbConfig>) -> Result<Self, EngineError> {
        let uid = uuid::Uuid::new_v4();
        let root_dir = config.root_dir.clone();
        let memetable_manager = build_memtable_manager(&config.memtable, Some(uid))?;
        // wal manager will handle its own recovery
        let wal_manager = build_wal_manger(&config.wal)?;
        // wal_manager.rotate(Some(uid))?;
        let sstable_root_dir = root_dir.join("sstable");
        // version manager will handle its own recovery
        let version_manager = Arc::new(RwLock::new(VersionManager::new(config.clone())?));
        // here will come the recovery process
        // read the entries from the wal and push them to memtable without any sstable push
        //  at the end we have active memtable and and immutable memtables in the memetable manager
        // then we can start the engine to serve the queries

        // now we know have both version manager and wal_manager now we will read the entries from wal manger and write it to engine
        let mut engine = Self {
            wal_manager: wal_manager,
            memtable_manager: memetable_manager,
            version_manager: version_manager.clone(),
            write_count: 0,
            metrics: Metrics::default(),
            workers: Vec::default(),
            under_shutdown: Arc::new(AtomicBool::new(false)),
        };
        let last_commited_offset = version_manager
            .read()?
            .get_latest_version()
            .get_commited_wal_offset();
        let entries = engine.wal_manager.read(last_commited_offset)?;
        for entry in entries {
            if let Ok((lsn, payload)) = entry {
                let entry = OwnedEntry::decode(&mut payload.as_slice())?;
                engine.memtable_manager.insert((&entry).into(), lsn)?;
            } else {
                panic!("Error while reading the wal")
            }
        }

        let compaction = build_compaction(
            &config.compaction,
            &config.bloom,
            &config.index,
            version_manager.clone(),
        );
        let under_shutdown = engine.under_shutdown.clone();
        engine.workers.push(thread::spawn(move || {
            loop {
                if under_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                    return 0;
                }
                sleep(Duration::from_millis(
                    config.compaction.compaction_interval as u64,
                ));
                if compaction.need_compaction() {
                    match compaction.run_compaction() {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error happen during the compaction {:?}", e)
                        }
                    }
                }
            }
        }));
        let under_shutdown = engine.under_shutdown.clone();

        let cleaner = Cleaner::new(version_manager.clone(), under_shutdown);
        engine.workers.push(cleaner.init());
        Ok(engine)
    }
    pub fn write(&mut self, e: Entry) -> Result<(), EngineError> {
        self.metrics.write_count += 1;
        if self.write_count % 500 == 0 {
            if self.memtable_manager.require_rotation() {
                self.memtable_rotation()?;
            }
            self.push_memetable()?;
        }
        let mut payload = Vec::new();
        e.encode(&mut payload)?;
        let wal_offset = self.wal_manager.append_log(&payload)?;
        self.memtable_manager.insert(e, wal_offset)?;
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
        Ok(())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.under_shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
        while let Some(worker) = self.workers.pop() {
            worker.join().unwrap();
        }
    }
}
