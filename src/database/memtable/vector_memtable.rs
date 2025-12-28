use std::collections::HashSet;

use uuid::Uuid;

use crate::database::memtable::{Entry, Memtable, errors::MemtableError};

pub struct VectorMemtable<K>
where
    K: Entry,
{
    id: Uuid,
    store: Vec<K>,
}

impl<K> VectorMemtable<K>
where
    K: Entry,
{
    pub fn new(id: Option<Uuid>) -> Self {
        Self {
            id: id.unwrap_or(uuid::Uuid::new_v4()),
            store: Vec::new(),
        }
    }
}

impl<K> Memtable<K> for VectorMemtable<K>
where
    K: Entry,
{
    fn get_id(&self) -> &Uuid {
        &self.id
    }
    fn insert(&mut self, e: K) {
        self.store.push(e);
    }
    fn delete(&mut self, mut e: K) {
        e.mark_deleted();
        self.store.push(e);
    }
    fn find(&self, key: &[u8]) -> Result<&K, MemtableError> {
        for element in self.store.iter().rev() {
            if element.get_key() == key {
                if element.is_deleted() {
                    return Err(MemtableError::Deleted);
                }
                return Ok(element);
            }
        }
        return Err(MemtableError::NotFound);
    }
    fn iter(&self) -> impl std::iter::Iterator<Item = &K> {
        VectorMemtableIterator {
            curr: self.store.len(),
            memtable: self,
            key_set: HashSet::default(),
        }
    }
}

pub(crate) struct VectorMemtableIterator<'a, K>
where
    K: Entry,
{
    memtable: &'a VectorMemtable<K>,
    curr: usize,
    key_set: HashSet<&'a [u8]>,
}
impl<'a, K> Iterator for VectorMemtableIterator<'a, K>
where
    K: Entry,
{
    type Item = &'a K;
    fn next(&mut self) -> Option<Self::Item> {
        while self.curr > 0 {
            self.curr -= 1;
            let curr_entry = &self.memtable.store[self.curr];
            if self.key_set.contains(curr_entry.get_key()) {
                continue;
            }
            self.key_set.insert(curr_entry.get_key());
            return Some(curr_entry);
        }
        return None;
    }
}
