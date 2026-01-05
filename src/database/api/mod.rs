use crate::database::{Entry, errors::DatabaseError};

pub struct Database {}
impl Database {
    pub fn write() -> Result<(), DatabaseError> {
        // write to wal
        // write to memtable
        // done
        Ok(())
    }
    pub fn read(key: &[u8]) -> Result<Entry, DatabaseError> {
        // read from active memtable
        // read from immutable memtable
        // read from sstables
        // done
        unimplemented!();
    }
    pub fn delete(key: &[u8]) -> Result<(), DatabaseError> {
        // write to wal
        // write to memtable
        // done
        Ok(())
    }
}
