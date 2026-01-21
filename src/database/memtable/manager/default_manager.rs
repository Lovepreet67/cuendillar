use std::collections::VecDeque;

use crate::database::memtable::{Memtable, errors::MemtableError, manager::MemtableManager};

pub struct DefaultManger<T>
where
    T: Memtable,
{
    active_memtable: T,
    immutable_memtables: VecDeque<T>,
    max_size: u64,
}

impl<T> DefaultManger<T>
where
    T: Memtable,
{
    pub fn intialize(active_memtable: T, immutable_memtables: VecDeque<T>, max_size: u64) -> Self {
        Self {
            active_memtable,
            immutable_memtables,
            max_size,
        }
    }
}

impl<T> MemtableManager for DefaultManger<T>
where
    T: Memtable,
{
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
    ) -> Result<(), crate::database::memtable::errors::MemtableError> {
        self.active_memtable.insert(e);
        Ok(())
    }
    fn rotate(
        &mut self,
        id: uuid::Uuid,
    ) -> Result<(), crate::database::memtable::errors::MemtableError> {
        let new_memtable = T::new(Some(id));
        let current_active_memtable = std::mem::replace(&mut self.active_memtable, new_memtable);
        self.immutable_memtables.push_front(current_active_memtable);
        Ok(())
    }
    fn iter(&self) -> impl std::iter::Iterator<Item = crate::database::Entry> {
        self.active_memtable.iter()
    }
    fn require_rotation(&self) -> bool {
        self.active_memtable.size() > self.max_size
    }
    fn get_memtable_to_push(&self) -> Option<&impl Memtable> {
        return self.immutable_memtables.back();
    }
    fn mark_pushed(&mut self, memetable_id: uuid::Uuid) -> Result<(), MemtableError> {
        if let Some(first_memtable) = self.immutable_memtables.front() {
            if first_memtable.get_id() != &memetable_id {
                return Err(MemtableError::InvalidCandidateId);
            }
            self.immutable_memtables.pop_back();
            return Ok(());
        }
        Err(MemtableError::NoImmutableMemtableExist)
    }
}
