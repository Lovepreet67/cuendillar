pub mod errors;
#[cfg(test)]
mod tests;

#[allow(dead_code)]
pub mod instrumented;

use std::{
    sync::{Arc, RwLock, atomic::AtomicBool},
    thread::{self, JoinHandle, sleep},
    time::Duration,
    u32,
};

use tracing::{error, info, instrument};

use crate::{DatabaseIterator, database::{
    Entry, OwnedEntry, config::DbConfig, db_engine::errors::EngineError, factory::{
        compaction::build_compaction, memtable::build_memtable_manager, wal::build_wal_manger,
    }, iterator::merged_iterator::MergedIterator, memtable::manager::MemtableManager, sstable::{
        cleaner::Cleaner,
        version::{ version_manager::VersionManager, version_update::VersionUpdate},
    }, wal::WAL
}};

pub struct Engine {
    config: Arc<DbConfig>,
    wal_manager: Arc<RwLock<Box<dyn WAL>>>,
    memtable_manager: Arc<RwLock<Box<dyn MemtableManager>>>,
    version_manager: Arc<RwLock<VersionManager>>,
    write_count: u64,
    workers: Vec<JoinHandle<u64>>,
    under_shutdown: Arc<AtomicBool>,
    pushing_memtable: Arc<AtomicBool>,
}

impl Engine {
    #[instrument(name="Engine Push Memtable",skip(self))]
    fn push_memetable(&mut self) -> Result<(), EngineError> {
        // we will fetch the immutable table from the memtable_manager
        let ready_to_push_memetable = self.memtable_manager.read(
            // "Getting ready to push memetable"
            )?.get_memtable_to_push();
        if ready_to_push_memetable.is_none() {
            return Ok(());
        }
        let ready_to_push_memetable = ready_to_push_memetable.unwrap();
        // now we will spun a thread
        let config = self.config.clone();
        let version_manager = self.version_manager.clone();
        let wal_manager = self.wal_manager.clone();
        let memtable_manager = self.memtable_manager.clone();
        let pushing_memtable = self.pushing_memtable.clone();
        let handler = thread::spawn(move || {
            let result: Result<(), String> = (|| {
                let sst_meta = VersionManager::push_memtable_static(
                    &config.sstable_root_dir,
                    &config.bloom,
                    &config.index,
                    ready_to_push_memetable.clone(),
                )
                .map_err(|e| format!("SST generation failed: {:?}", e))?;
                let mut version_update = VersionUpdate::new(ready_to_push_memetable.get_wal_offset());
                version_update.add_operation(
                    crate::database::sstable::version::version_update::VersionOperation::AddWithMeta 
                    { level: 0, meta: sst_meta, index: u32::MAX }
                );
                
                version_manager
                    .write(
                        // "Version manager while pushing version update in push memtable"
                    )
                    .map_err(|e| format!("VersionManager lock poisoned: {:?}", e))?
                    .push_version_update( version_update)
                    .map_err(|e| format!("Error while pushing the new version update: {:?}", e))?;
                

                wal_manager
                    .write(
                        // "WAL manager in push memtable"
                        )
                    .map_err(|e| format!("WALManager lock poisoned: {:?}", e))?
                    .flush_wal(ready_to_push_memetable.get_wal_offset())
                    .map_err(|e| format!("WAL flush failed: {:?}", e))?;
                

                memtable_manager
                    .write(
                        // "memtable manager in push memtable"
                    )
                    .map_err(|e| format!("MemtableManager lock poisoned: {:?}", e))?
                    .mark_pushed(ready_to_push_memetable.get_id().clone())
                    .map_err(|e| format!("Mark pushed failed: {:?}", e))?;
                Ok(())
            })();

            if let Err(err) = result {
                error!("Background memtable push failed: {}", err);
            }

            pushing_memtable.store(false, std::sync::atomic::Ordering::Release);
            return 0;
        });

        self.workers.push(handler);
        return Ok(());
    }
    #[instrument(name="Engine New",skip(config))]
    pub fn new(config: Arc<DbConfig>) -> Result<Self, EngineError> {
        let uid = uuid::Uuid::new_v4();
        let memetable_manager = build_memtable_manager(&config.memtable, Some(uid))?;
        // wal manager will handle its own recovery
        let wal_manager = build_wal_manger(&config.wal)?;
        // wal_manager.rotate(Some(uid))?;
        // version manager will handle its own recovery
         // now we know have both version manager and wal_manager now we will read the entries from wal manger and write it to engine
        let (cleaner_channel_producer, cleaner_channel_receiver) = std::sync::mpsc::channel();

        let version_manager = Arc::new(RwLock::new(
            // "Verions manger", 
            VersionManager::new(config.clone(),cleaner_channel_producer)?));
        // here will come the recovery process
        // read the entries from the wal and push them to memtable without any sstable push
        //  at the end we have active memtable and and immutable memtables in the memetable manager
        // then we can start the engine to serve the queries

       
        let mut engine = Self {
            config: config.clone(),
            wal_manager: Arc::new(RwLock::new(
                // "WAL manger",
                wal_manager
                )),
            memtable_manager: Arc::new(RwLock::new(
                // "Memtable manger",
                memetable_manager)),
            version_manager: version_manager.clone(),
            write_count: 0,
            workers: Vec::default(),
            under_shutdown: Arc::new(AtomicBool::new(false)),
            pushing_memtable: Arc::new(AtomicBool::new(false)),
        };
        let last_commited_offset = version_manager
            .read(
                // "Getting last commited offset from version manger in new"
                )?
            .get_latest_version()
            .get_commited_wal_offset();
        info!("Last Committed WAL offset {}",last_commited_offset);
        let engine_wal_manager = engine.wal_manager.read(
            // "During new engine"
            )?;
        let mut engine_memtable_manager = engine.memtable_manager.write(
            // "During new engine"
            )?;
        let entries = engine_wal_manager.read(last_commited_offset)?;
        for entry in entries {
            if let Ok((lsn, payload)) = entry {
                let entry = OwnedEntry::decode(&mut payload.as_slice())?;
                engine_memtable_manager.insert((&entry).into(), lsn)?;
            } else {
                panic!("Error while reading the wal")
            }
        }
        info!("Successfully Replayed the WAL and inserted data in memetable");

        let compaction = build_compaction(
            &config.compaction,
            &config.bloom,
            &config.index,
            version_manager.clone(),
        );
        let under_shutdown = engine.under_shutdown.clone();
        let compaction_interval = config.compaction.compaction_interval;
        engine.workers.push(thread::spawn(move || {
            loop {
                if under_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                    return 0;
                }
                sleep(Duration::from_millis(compaction_interval as u64));
                if compaction.need_compaction() {
                    match compaction.run_compaction() {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error happen during the compaction {:?}", e)
                        }
                    }
                }
            }
        }));
        let under_shutdown = engine.under_shutdown.clone();

        let clearner_config = config.cleaning.clone();

        let cleaner = Cleaner::new(clearner_config, under_shutdown, cleaner_channel_receiver);
        engine.workers.push(cleaner.init());
        drop(engine_memtable_manager);
        drop(engine_wal_manager);
        Ok(engine)
    }
    #[instrument(name="Engine Write",skip(self))]
    pub fn write(&mut self, key:&[u8],value:&[u8]) -> Result<u64, EngineError> {
        if self.memtable_manager.read(
            // "Checking if the rotaion is required"
        )?.require_rotation() {
            self.memtable_rotation()?;
            if !self
                .pushing_memtable
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                self.pushing_memtable
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                self.push_memetable()?;
            }
        }

        let mut wal_manager = self.wal_manager.write()?;
        let seq_no = wal_manager.get_offset();
        let e = Entry::from_key_value(seq_no, key, value);
        let mut payload = Vec::new();
        e.encode(&mut payload)?;
        let wal_offset = wal_manager.append_log(&payload)?;
        drop(wal_manager);
        
        self.memtable_manager.write(
            // "During Writing"
            )?.insert(e, wal_offset)?;
        self.write_count += 1;
        Ok(seq_no)
    }
    #[instrument(name="Engine Find",skip(self))]
    pub fn find(&self, key: &[u8]) -> Result<Option<OwnedEntry>, EngineError> {
        let result = self.memtable_manager.read(
            // "During find"
            )?.find(key)?.map(|x| x.into());
        if result.is_none() {
            let latest_version = self.version_manager.read(
                // "During find"
                )?.get_latest_version();
            let sstable_result = latest_version.find(key)?;
            return Ok(sstable_result);
        }
        Ok(match result {
            Some(x) => Some(x),
            None => None,
        })
    }
    pub fn iterator(&self,start_key:Option<&[u8]>,end_key:Option<&[u8]>)->Result<Box<dyn DatabaseIterator>,EngineError>{
        let mut mi = MergedIterator::new();
        let memtable_iter = self.memtable_manager.read()?.iter(start_key,end_key);
        mi.add_iterator(Box::new(memtable_iter));
        let version_iter = self.version_manager.read()?.iter(start_key,end_key)?;
        mi.add_iterator(Box::new(version_iter));
        Ok(Box::new(mi))
    }
    fn memtable_rotation(&mut self) -> Result<(), EngineError> {
        let uid = uuid::Uuid::new_v4();
        self.memtable_manager.write(
            // "During rotation"
            )?.rotate(uid)?;
        Ok(())
    }
}

impl Drop for Engine {
    #[instrument(name="Engine Stop",skip(self))]
    fn drop(&mut self) {
        info!("Stoping Engine....");
        // we will first fetch the compaction and cleaner workers
        let compaction_worker =  self.workers.pop();
        let cleaner_worker = self.workers.pop();

        // first we let all the workers to push memtables
        while let Some(worker) = self.workers.pop() {
            worker.join().unwrap();
        }
        // then we will let compaction to run for 120 secs and then we will singal it to stop
        // sleep(Duration::from_secs(120));
        self.under_shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(compaction_worker) = compaction_worker{
            info!("Waiting for compaction to stop");
            compaction_worker.join().expect("Error while joining compaction thread");
        }
        // then we will wait for 30 secs so that cleaner can do its job 
        // sleep(Duration::from_millis(self.config.cleaning.cleaning_interval as u64*10));
        if let Some(clearner_worker) = cleaner_worker{
            info!("Waiting for cleaner to stop");
            clearner_worker.join().expect("Error while joining the cleaner thread");
        }
    }
}
