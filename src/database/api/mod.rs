use std::marker::PhantomData;

use crate::database::{common::Entry, errors::DatabaseError};

pub struct Database<T>
where
    T: Entry,
{
    x: PhantomData<T>,
}
impl<T> Database<T>
where
    T: Entry,
{
    pub fn write() -> Result<(), DatabaseError> {
        // write to wal
        // write to memtable
        // done
        Ok(())
    }
    pub fn read(key: &[u8]) -> Result<T, DatabaseError> {
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
