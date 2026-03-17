use std::{collections::VecDeque, sync::Arc};

use tracing::instrument;

use crate::database::{
    iterator::merged_iterator::MergedIterator,
    memtable::{Memtable, errors::MemtableError, manager::MemtableManager},
};

pub struct DefaultManger {
    active_memtable: Box<dyn Memtable>,
    immutable_memtables: VecDeque<Arc<dyn Memtable>>,
    max_size: u64,
    memtable_generator: Arc<dyn Fn(Option<uuid::Uuid>) -> Box<dyn Memtable> + Send + Sync>,
}

impl DefaultManger {
    pub fn intialize(
        active_memtable: Box<dyn Memtable>,
        immutable_memtables: VecDeque<Arc<dyn Memtable>>,
        max_size: u64,
        memtable_generator: Arc<dyn Fn(Option<uuid::Uuid>) -> Box<dyn Memtable> + Send + Sync>,
    ) -> Self {
        Self {
            active_memtable,
            immutable_memtables,
            max_size,
            memtable_generator,
        }
    }
}

impl MemtableManager for DefaultManger {
    #[instrument(name = "Default Memetable Manger Find", skip(self))]
    fn find(
        &self,
        key: &[u8],
    ) -> Result<Option<crate::database::Entry<'_>>, crate::database::memtable::errors::MemtableError>
    {
        // first we will check in the active memtable
        if let Some(val) = self.active_memtable.find(key)? {
            return Ok(Some(val));
        }
        for m_t in self.immutable_memtables.iter() {
            if let Some(val) = m_t.find(key)? {
                return Ok(Some(val));
            }
        }
        return Ok(None);
    }
    #[instrument(name = "Default Memetable  Manger insert", skip(self))]
    fn insert(
        &mut self,
        e: crate::database::Entry<'_>,
        wal_offset: u64,
    ) -> Result<(), crate::database::memtable::errors::MemtableError> {
        self.active_memtable.insert(e, wal_offset);
        Ok(())
    }
    fn rotate(
        &mut self,
        id: uuid::Uuid,
    ) -> Result<(), crate::database::memtable::errors::MemtableError> {
        let new_memtable = (self.memtable_generator)(Some(id));
        let current_active_memtable =
            std::mem::replace(&mut self.active_memtable, new_memtable).into();
        self.immutable_memtables.push_front(current_active_memtable);
        Ok(())
    }
    fn iter(&self, start_key: Option<&[u8]>, end_key: Option<&[u8]>) -> MergedIterator {
        // we will create all the iterators for tables and push them to merge iterator
        let mut mi = MergedIterator::new();
        // now we will push the iterators one by one
        mi.add_iterator(self.active_memtable.iter(start_key, end_key));
        for table in &self.immutable_memtables {
            mi.add_iterator(table.iter(start_key, end_key));
        }
        mi
        // Box::new(self.active_memtable.iter())
    }
    fn require_rotation(&self) -> bool {
        self.active_memtable.size() > self.max_size * 1000_000 // as max size in MB and size is in Bytes
    }
    fn get_memtable_to_push(&self) -> Option<Arc<dyn Memtable>> {
        return self.immutable_memtables.back().map(|x| x.clone());
    }
    fn mark_pushed(&mut self, memetable_id: uuid::Uuid) -> Result<(), MemtableError> {
        if let Some(first_memtable) = self.immutable_memtables.back() {
            if first_memtable.get_id() != &memetable_id {
                return Err(MemtableError::InvalidCandidateId);
            }
        } else {
            return Err(MemtableError::NoImmutableMemtableExist);
        }
        self.immutable_memtables.pop_back();
        return Ok(());
    }
}
