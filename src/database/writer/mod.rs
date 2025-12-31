use std::{marker::PhantomData, path::PathBuf};

use uuid::Uuid;

use crate::database::{common::Entry, memtable::Memtable, wal::wal_writer::WALWriter};
#[cfg(test)]
mod tests;

pub struct Writer<T, M, W>
where
    T: Entry,
    M: Memtable<T>,
    W: WALWriter,
{
    t: PhantomData<T>,
    active_memtable: Option<M>,
    immutable_memtable: Vec<M>,
    wal_writer: W,
    memtable_memory_threshold: Option<u64>,
    memtable_num_entery_threshold: Option<u64>,
}

impl<T, M, W> Writer<T, M, W>
where
    T: Entry,
    M: Memtable<T>,
    W: WALWriter,
{
    pub fn new(
        wal_root_dir: PathBuf,
        memtable_memory_threshold: Option<u64>,
        memtable_num_entery_threshold: Option<u64>,
    ) -> Self {
        // startup should happen here and read the presisted state
        Self {
            t: PhantomData::default(),
            active_memtable: None,
            immutable_memtable: Vec::default(),
            // TODO: Remove this unwrap
            wal_writer: W::new(wal_root_dir).unwrap(),
            memtable_memory_threshold,
            memtable_num_entery_threshold,
        }
    }
    fn memtable_rotation(&mut self) {
        if let Some(active_memtable) = self.active_memtable.take() {
            self.immutable_memtable.push(active_memtable);
        }
        let memtable_id = Uuid::new_v4();
        self.active_memtable = Some(M::new(Some(memtable_id)));
        self.wal_writer.rotate(Some(memtable_id));
    }
    pub fn write(&mut self, entry: T) {
        if self.active_memtable.is_none() {
            self.memtable_rotation();
        }
        assert!(self.active_memtable.is_some());
        let active_memtable = self.active_memtable.as_mut().unwrap();

        // for now we will check for threshold here
        let need_rotation = match (
            self.memtable_memory_threshold,
            self.memtable_num_entery_threshold,
        ) {
            (Some(memory), Some(num_entry)) => {
                memory < active_memtable.size() || num_entry < active_memtable.num_enteries()
            }
            (Some(memory), None) => memory < active_memtable.size(),
            (None, Some(num_enteries)) => num_enteries < active_memtable.num_enteries(),
            (None, None) => false,
        };
        if need_rotation {
            self.memtable_rotation();
        }
        let active_memtable = self.active_memtable.as_mut().unwrap();

        // now we will write
        self.wal_writer
            .append(crate::database::wal::wal_entry::WALEntry::from_entry(
                &entry,
            ));
        active_memtable.insert(entry);
    }
}
