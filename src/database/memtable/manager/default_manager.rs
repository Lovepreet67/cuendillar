use std::collections::VecDeque;

use crate::database::memtable::{Memtable, errors::MemtableError, manager::MemtableManager};

pub struct DefaultManger {
    active_memtable: Box<dyn Memtable>,
    immutable_memtables: VecDeque<Box<dyn Memtable>>,
    max_size: u64,
    memtable_generator: Box<dyn Fn(Option<uuid::Uuid>) -> Box<dyn Memtable>>,
}

impl DefaultManger {
    pub fn intialize(
        active_memtable: Box<dyn Memtable>,
        immutable_memtables: VecDeque<Box<dyn Memtable>>,
        max_size: u64,
        memtable_generator: Box<dyn Fn(Option<uuid::Uuid>) -> Box<dyn Memtable>>,
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
        let current_active_memtable = std::mem::replace(&mut self.active_memtable, new_memtable);
        self.immutable_memtables.push_front(current_active_memtable);
        Ok(())
    }
    fn iter(&self) -> Box<dyn std::iter::Iterator<Item = crate::database::Entry<'_>> + '_> {
        Box::new(self.active_memtable.iter())
    }
    fn require_rotation(&self) -> bool {
        self.active_memtable.size() > self.max_size
    }
    fn get_memtable_to_push(&self) -> Option<&dyn Memtable> {
        return self.immutable_memtables.back().map(|b| b.as_ref());
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
