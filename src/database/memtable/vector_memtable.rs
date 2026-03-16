use std::{collections::HashSet, sync::Arc};

use tracing::instrument;
use uuid::Uuid;

use crate::{
    OwnedEntry,
    database::{
        Entry,
        iterator::DatabaseIterator,
        memtable::{Memtable, errors::MemtableError},
    },
};
#[derive(Clone)]
pub struct VectorMemtableEntry {
    seq_no: u64,
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
            Entry::Row { seq_no, key, value } => Self {
                seq_no,
                key: key.into(),
                value: Some(value.into()),
            },
            Entry::Tombstone { seq_no, key } => Self {
                seq_no,
                key: key.into(),
                value: None,
            },
        };
    }
}
impl<'a> From<&'a VectorMemtableEntry> for Entry<'a> {
    fn from(value: &'a VectorMemtableEntry) -> Self {
        if value.is_deleted() {
            return Entry::Tombstone {
                seq_no: value.seq_no,
                key: &value.key,
            };
        } else {
            return Entry::Row {
                seq_no: value.seq_no,
                key: &value.key,
                value: value.value.as_deref().unwrap(),
            };
        }
    }
}
impl<'a> From<&'a VectorMemtableEntry> for OwnedEntry {
    fn from(value: &'a VectorMemtableEntry) -> Self {
        if value.is_deleted() {
            return OwnedEntry::Tombstone {
                seq_no: value.seq_no,
                key: value.key.clone(),
            };
        } else {
            return OwnedEntry::Row {
                seq_no: value.seq_no,
                key: value.key.clone(),
                value: value.value.clone().unwrap(),
            };
        }
    }
}
#[derive(Clone)]
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
    #[instrument(name = "Vector Memetable Insert", skip(self))]
    fn insert(&mut self, e: Entry, wal_offset: u64) {
        self.wal_offset = wal_offset;
        let memtable_entry: VectorMemtableEntry = e.into();
        self.curr_size += memtable_entry.size() as u64;
        self.store.push(memtable_entry);
    }
    #[instrument(name = "Vector Memetable Find", skip(self))]
    fn find(&self, key: &[u8]) -> Result<Option<Entry<'_>>, MemtableError> {
        for element in self.store.iter().rev() {
            if element.get_key() == key {
                return Ok(Some(element.into()));
            }
        }
        return Ok(None);
    }
    #[instrument(name = "Vector Memetable Iter", skip(self))]
    fn iter(&self) -> Box<dyn DatabaseIterator> {
        // we will store a copy of enteries in sorted order
        let mut seen = HashSet::new();
        let mut entries = Vec::new();

        for (idx, entry) in self.store.iter().enumerate().rev() {
            if seen.insert(entry.key.clone()) {
                entries.push(idx);
            }
        }

        entries.sort_by(|a, b| self.store[*a].key.cmp(&self.store[*b].key));

        Box::new(VectorMemtableIterator {
            memtable: Arc::new(self.clone()),
            entries,
            curr: 0,
        })
    }
    fn num_enteries(&self) -> u64 {
        self.store.len() as u64
    }
    fn size(&self) -> u64 {
        self.curr_size
    }
    fn get_wal_offset(&self) -> u64 {
        self.wal_offset
    }
}

pub(crate) struct VectorMemtableIterator {
    memtable: Arc<VectorMemtable>,
    entries: Vec<usize>,
    curr: usize,
}

impl DatabaseIterator for VectorMemtableIterator {
    fn peek(&self) -> Option<Entry<'_>> {
        if self.curr >= self.entries.len() {
            return None;
        }
        let idx = self.entries[self.curr];
        Some((&self.memtable.store[idx]).into())
    }
    fn next_owned(&mut self) -> Option<crate::OwnedEntry> {
        if self.curr >= self.entries.len() {
            return None;
        }

        let idx = self.entries[self.curr];
        self.curr += 1;

        Some((&self.memtable.store[idx]).into())
    }
    fn first_entry(&self) -> Option<Entry<'_>> {
        self.entries
            .first()
            .map(|idx| (&self.memtable.store[*idx]).into())
    }
    fn last_entry(&self) -> Option<Entry<'_>> {
        self.entries
            .last()
            .map(|idx| (&self.memtable.store[*idx]).into())
    }
}
