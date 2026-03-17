use std::collections::HashMap;

use tracing::instrument;
use uuid::Uuid;

use crate::{
    OwnedEntry,
    database::{Entry, iterator::DatabaseIterator, memtable::Memtable},
};

/// NOT PERFORMING GOOD
pub struct HashMetable {
    id: Uuid,
    store: HashMap<Vec<u8>, (u64, Option<Vec<u8>>)>,
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
        hashtable_entry: Option<(&'a Vec<u8>, &'a (u64, Option<Vec<u8>>))>,
    ) -> Option<Entry<'a>> {
        match hashtable_entry {
            Some((key, (seq_no, Some(value)))) => Some(Entry::Row {
                seq_no: *seq_no,
                key,
                value,
            }),
            Some((key, (seq_no, None))) => Some(Entry::Tombstone {
                seq_no: *seq_no,
                key,
            }),
            None => None,
        }
    }
    fn get_owned_entry_from_hashtable_entry<'a>(
        hashtable_entry: Option<(&'a Vec<u8>, &'a (u64, Option<Vec<u8>>))>,
    ) -> Option<OwnedEntry> {
        match hashtable_entry {
            Some((key, (seq_no, Some(value)))) => Some(OwnedEntry::Row {
                seq_no: *seq_no,
                key: key.clone(),
                value: value.clone(),
            }),
            Some((key, (seq_no, None))) => Some(OwnedEntry::Tombstone {
                seq_no: *seq_no,
                key: key.clone(),
            }),
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
            Entry::Row { seq_no, key, value } => {
                self.store.insert(key.into(), (seq_no, Some(value.into())));
                self.curr_size += key.len() as u64;
                self.curr_size += value.len() as u64;
            }
            Entry::Tombstone { seq_no, key } => {
                self.store.insert(key.into(), (seq_no, None));
                self.curr_size += key.len() as u64;
            }
        };
    }
    #[instrument(name = "Hash Memetable Iter", skip(self))]
    fn iter(&self, start_key: Option<&[u8]>, end_key: Option<&[u8]>) -> Box<dyn DatabaseIterator> {
        let mut entries: Vec<OwnedEntry> = self
            .store
            .iter()
            .filter(|item| {
                let in_range = start_key.map_or(true, |start| item.0.as_slice() >= start)
                    && end_key.map_or(true, |end| item.0.as_slice() <= end);
                in_range
            })
            .map(|item| Self::get_owned_entry_from_hashtable_entry(Some(item)).unwrap())
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

pub(crate) struct HashMetableIterator {
    entries: Vec<OwnedEntry>,
    curr: usize,
}

impl DatabaseIterator for HashMetableIterator {
    fn next_owned(&mut self) -> Option<OwnedEntry> {
        if self.curr >= self.entries.len() {
            None
        } else {
            let e = self.entries[self.curr].clone();
            self.curr += 1;
            Some(e.into())
        }
    }
    fn peek(&self) -> Option<Entry<'_>> {
        if self.entries.len() > self.curr {
            return Some((&self.entries[self.curr]).into());
        } else {
            None
        }
    }
    fn first_entry(&self) -> Option<Entry<'_>> {
        if self.entries.len() > 0 {
            Some((&self.entries[0]).into())
        } else {
            None
        }
    }
    fn last_entry(&self) -> Option<Entry<'_>> {
        if self.entries.len() > 0 {
            Some((&self.entries[self.entries.len() - 1]).into())
        } else {
            None
        }
    }
}
