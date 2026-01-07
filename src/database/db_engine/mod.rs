mod errors;
#[cfg(test)]
mod tests;
use std::{collections::VecDeque, error::Error, path::PathBuf, str::FromStr};

use crate::database::{
    Entry,
    db_engine::errors::EngineError,
    memtable::{
        Memtable,
        manager::{
            MemtableManager,
            default_manager::{self, DefaultManger},
        },
        vector_memtable::VectorMemtable,
    },
    wal::{self, WAL, default_wal::DefaultWAL, wal_entry::WALEntry},
};

pub struct Engine {
    wal_manager: DefaultWAL,
    memtable_manager: DefaultManger<VectorMemtable>,
}
impl Engine {
    pub fn new(root_path: &str) -> Result<Self, Box<dyn Error>> {
        let uid = uuid::Uuid::new_v4();
        let first_memtable = VectorMemtable::new(Some(uid));
        let memetable_manager = DefaultManger::intialize(first_memtable, VecDeque::new());
        let mut wal_manager = DefaultWAL::new(PathBuf::from_str(root_path)?).unwrap();
        wal_manager.rotate(Some(uid));
        Ok(Self {
            wal_manager: wal_manager,
            memtable_manager: memetable_manager,
        })
    }
    pub fn write(&mut self, e: Entry) -> Result<(), EngineError> {
        self.wal_manager.append_log(WALEntry::from_entry(&e))?;
        self.memtable_manager.insert(e)?;
        Ok(())
    }
    pub fn find(&mut self, key: &[u8]) -> Result<Option<Entry>, EngineError> {
        let result = self.memtable_manager.find(key)?;
        Ok(result)
    }
}
