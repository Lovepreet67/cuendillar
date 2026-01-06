use std::collections::HashSet;

use uuid::Uuid;

use crate::database::{
    Entry,
    memtable::{Memtable, errors::MemtableError},
};
pub struct VectorMemtableEntry {
    key: Vec<u8>,
    value: Option<Vec<u8>>,
}

impl VectorMemtableEntry {
    pub fn get_key(&self) -> &[u8] {
        &self.key
    }
    pub fn is_deleted(&self) -> bool {
        self.value.is_none()
    }
    pub fn size(&self) -> usize {
        self.key.len()
            + match &self.value {
                Some(val) => val.len(),
                None => 0,
            }
    }
}

impl From<Entry<'_>> for VectorMemtableEntry {
    fn from(value: Entry) -> Self {
        return match value {
            Entry::Row { key, value } => Self {
                key: key.into(),
                value: Some(value.into()),
            },
            Entry::Tombstone { key } => Self {
                key: key.into(),
                value: None,
            },
        };
    }
}
impl<'a> From<&'a VectorMemtableEntry> for Entry<'a> {
    fn from(value: &'a VectorMemtableEntry) -> Self {
        if value.is_deleted() {
            return Entry::Tombstone { key: &value.key };
        } else {
            return Entry::Row {
                key: &value.key,
                value: value.value.as_deref().unwrap(),
            };
        }
    }
}
pub struct VectorMemtable {
    id: Uuid,
    store: Vec<VectorMemtableEntry>,
}

impl Memtable for VectorMemtable {
    fn new(id: Option<Uuid>) -> Self {
        Self {
            id: id.unwrap_or_else(|| Uuid::new_v4()),
            store: Vec::new(),
        }
    }
    fn get_id(&self) -> &Uuid {
        &self.id
    }
    fn insert(&mut self, e: Entry) {
        self.store.push(e.into());
    }
    fn find(&self, key: &[u8]) -> Result<Option<Entry>, MemtableError> {
        for element in self.store.iter().rev() {
            if element.get_key() == key {
                return Ok(Some(element.into()));
            }
        }
        return Ok(None);
    }
    fn iter(&self) -> impl std::iter::Iterator<Item = Entry<'_>> {
        VectorMemtableIterator {
            curr: self.store.len(),
            memtable: self,
            key_set: HashSet::default(),
        }
    }
    fn num_enteries(&self) -> u64 {
        self.store.len() as u64
    }
    fn size(&self) -> u64 {
        let mut totalSize = 0;
        for i in &self.store {
            totalSize += i.size();
        }
        totalSize as u64
    }
}

pub(crate) struct VectorMemtableIterator<'a> {
    memtable: &'a VectorMemtable,
    curr: usize,
    key_set: HashSet<&'a [u8]>,
}
impl<'a> Iterator for VectorMemtableIterator<'a> {
    type Item = Entry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        while self.curr > 0 {
            self.curr -= 1;
            let curr_entry = &self.memtable.store[self.curr];
            if self.key_set.contains(curr_entry.get_key()) {
                continue;
            }
            self.key_set.insert(curr_entry.get_key());
            return Some(curr_entry.into());
        }
        return None;
    }
}
