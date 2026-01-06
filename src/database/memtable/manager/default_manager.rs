use std::collections::VecDeque;

use crate::database::memtable::{Memtable, manager::MemtableManager};

pub struct DefaultManger<T>
where
    T: Memtable,
{
    active_memtable: T,
    immutable_memtables: VecDeque<T>,
}

impl<T> DefaultManger<T>
where
    T: Memtable,
{
    pub fn intialize(active_memtable: T, immutable_memtables: VecDeque<T>) -> Self {
        Self {
            active_memtable,
            immutable_memtables,
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
}
