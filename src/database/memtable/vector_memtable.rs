use std::collections::HashSet;

use uuid::Uuid;

use crate::database::{
    Entry,
    memtable::{Memtable, MemtableIterator, errors::MemtableError},
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
    wal_offset: u64,
    curr_size: u64,
    store: Vec<VectorMemtableEntry>,
}
impl VectorMemtable {
    pub fn new(id: Option<Uuid>) -> Self {
        Self {
            curr_size: 0,
            wal_offset: 0,
            id: id.unwrap_or_else(|| Uuid::new_v4()),
            store: Vec::new(),
        }
    }
}
impl Memtable for VectorMemtable {
    fn get_id(&self) -> &Uuid {
        &self.id
    }
    fn insert(&mut self, e: Entry, wal_offset: u64) {
        self.wal_offset = wal_offset;
        let memtable_entry: VectorMemtableEntry = e.into();
        self.curr_size += memtable_entry.size() as u64;
        self.store.push(memtable_entry);
    }
    fn find(&self, key: &[u8]) -> Result<Option<Entry<'_>>, MemtableError> {
        for element in self.store.iter().rev() {
            if element.get_key() == key {
                return Ok(Some(element.into()));
            }
        }
        return Ok(None);
    }
    fn iter(&self) -> Box<dyn MemtableIterator<Item = Entry<'_>> + '_> {
        // we will store a copy of enteries in sorted order
        let mut seen = HashSet::new();
        let mut entries = Vec::new();

        for entry in self.store.iter().rev() {
            if seen.insert(entry.key.clone()) {
                // first time seeing this key = latest version
                entries.push(entry);
            }
        }

        entries.sort_by(|a, b| a.key.cmp(&b.key));

        Box::new(VectorMemtableIterator { curr: 0, entries })
    }
    fn num_enteries(&self) -> u64 {
        self.store.len() as u64
    }
    fn size(&self) -> u64 {
        // self.curr_size
        self.store.len() as u64
    }
    fn get_wal_offset(&self) -> u64 {
        self.wal_offset
    }
}

pub(crate) struct VectorMemtableIterator<'a> {
    entries: Vec<&'a VectorMemtableEntry>,
    curr: usize,
}
impl<'a> Iterator for VectorMemtableIterator<'a> {
    type Item = Entry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.curr >= self.entries.len() {
            None
        } else {
            let e = self.entries[self.curr];
            self.curr += 1;
            Some(e.into())
        }
    }
}

impl<'a> MemtableIterator for VectorMemtableIterator<'a> {
    fn get_first_entry(&self) -> Option<Entry<'_>> {
        if self.entries.len() > 0 {
            Some(self.entries[0].into())
        } else {
            None
        }
    }
    fn get_last_entry(&self) -> Option<Entry<'_>> {
        if self.entries.len() > 0 {
            Some(self.entries[self.entries.len() - 1].into())
        } else {
            None
        }
    }
}
