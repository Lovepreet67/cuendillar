use std::collections::HashMap;

use tracing::instrument;
use uuid::Uuid;

use crate::database::{
    Entry,
    memtable::{Memtable, MemtableIterator},
};

/// NOT PERFORMING GOOD
pub struct HashMetable {
    id: Uuid,
    store: HashMap<Vec<u8>, Option<Vec<u8>>>,
    curr_size: u64,
    wal_offset: u64,
}
impl HashMetable {
    pub fn new(id: Option<Uuid>) -> Self {
        Self {
            id: id.unwrap_or_else(|| Uuid::new_v4()),
            store: HashMap::new(),
            curr_size: 0,
            wal_offset: 0,
        }
    }
    fn get_entry_from_hashtable_entry<'a>(
        hashtable_entry: Option<(&'a Vec<u8>, &'a Option<Vec<u8>>)>,
    ) -> Option<Entry<'a>> {
        match hashtable_entry {
            Some((key, Some(value))) => Some(Entry::Row { key, value }),
            Some((key, None)) => Some(Entry::Tombstone { key }),
            None => None,
        }
    }
}

impl Memtable for HashMetable {
    fn get_id(&self) -> &uuid::Uuid {
        &self.id
    }
    fn get_wal_offset(&self) -> u64 {
        self.wal_offset
    }
    #[instrument(name = "Hash Memetable Find", skip(self))]
    fn find(
        &self,
        key: &[u8],
    ) -> Result<Option<crate::database::Entry<'_>>, super::errors::MemtableError> {
        Ok(Self::get_entry_from_hashtable_entry(
            self.store.get_key_value(key),
        ))
    }
    #[instrument(name = "Hash Memetable Insert", skip(self))]
    fn insert(&mut self, e: crate::database::Entry, wal_offset: u64) {
        self.wal_offset = wal_offset;
        match e {
            Entry::Row { key, value } => {
                self.store.insert(key.into(), Some(value.into()));
                self.curr_size += key.len() as u64;
                self.curr_size += value.len() as u64;
            }
            Entry::Tombstone { key } => {
                self.store.insert(key.into(), None);
                self.curr_size += key.len() as u64;
            }
        };
    }
    #[instrument(name = "Hash Memetable Iter", skip(self))]
    fn iter(&self) -> Box<dyn super::MemtableIterator<Item = crate::database::Entry<'_>> + '_> {
        let mut entries: Vec<Entry<'_>> = self
            .store
            .iter()
            .map(|item| Self::get_entry_from_hashtable_entry(Some(item)).unwrap())
            .collect();
        entries.sort_by(|a, b| a.get_key().cmp(&b.get_key()));
        Box::new(HashMetableIterator { entries, curr: 0 })
    }

    fn num_enteries(&self) -> u64 {
        self.store.len() as u64
    }
    fn size(&self) -> u64 {
        self.curr_size
    }
}

pub(crate) struct HashMetableIterator<'a> {
    entries: Vec<Entry<'a>>,
    curr: usize,
}
impl<'a> Iterator for HashMetableIterator<'a> {
    type Item = Entry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.curr >= self.entries.len() {
            None
        } else {
            let e = self.entries[self.curr].clone();
            self.curr += 1;
            Some(e.into())
        }
    }
}

impl<'a> MemtableIterator for HashMetableIterator<'a> {
    fn get_first_entry(&self) -> Option<Entry<'_>> {
        if self.entries.len() > 0 {
            Some(self.entries[0].clone())
        } else {
            None
        }
    }
    fn get_last_entry(&self) -> Option<Entry<'_>> {
        if self.entries.len() > 0 {
            Some(self.entries[self.entries.len() - 1].clone())
        } else {
            None
        }
    }
}
