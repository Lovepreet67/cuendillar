use std::collections::{BTreeMap, btree_map::Iter};

use tracing::instrument;
use uuid::Uuid;

use crate::database::{
    Entry,
    memtable::{Memtable, MemtableIterator},
};

pub struct BTreeMemtable {
    id: Uuid,
    store: BTreeMap<Vec<u8>, (u64, Option<Vec<u8>>)>,
    curr_size: u64,
    wal_offset: u64,
}
impl BTreeMemtable {
    pub fn new(id: Option<Uuid>) -> Self {
        Self {
            id: id.unwrap_or_else(|| Uuid::new_v4()),
            store: BTreeMap::new(),
            curr_size: 0,
            wal_offset: 0,
        }
    }
    fn get_entry_from_btree_entry<'a>(
        btree_entry: Option<(&'a Vec<u8>, &'a (u64, Option<Vec<u8>>))>,
    ) -> Option<Entry<'a>> {
        match btree_entry {
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
}

impl Memtable for BTreeMemtable {
    fn get_id(&self) -> &uuid::Uuid {
        &self.id
    }
    fn get_wal_offset(&self) -> u64 {
        self.wal_offset
    }
    #[instrument(name = "BTree Memetable Find", skip(self))]
    fn find(
        &self,
        key: &[u8],
    ) -> Result<Option<crate::database::Entry<'_>>, super::errors::MemtableError> {
        Ok(Self::get_entry_from_btree_entry(
            self.store.get_key_value(key),
        ))
    }
    #[instrument(name = "BTree Memetable Insert", skip(self))]
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
    #[instrument(name = "BTree Memetable Iter", skip(self))]
    fn iter(&self) -> Box<dyn super::MemtableIterator<Item = crate::database::Entry<'_>> + '_> {
        let first_entry = Self::get_entry_from_btree_entry(self.store.first_key_value());
        let last_entry = Self::get_entry_from_btree_entry(self.store.last_key_value());
        let iterator = self.store.iter();
        Box::new(BTreeMemtableIterator::new(
            first_entry,
            last_entry,
            iterator,
        ))
    }
    fn num_enteries(&self) -> u64 {
        self.store.len() as u64
    }
    fn size(&self) -> u64 {
        self.curr_size
    }
}

pub struct BTreeMemtableIterator<'a> {
    first_entry: Option<Entry<'a>>,
    last_entry: Option<Entry<'a>>,
    iterator: Iter<'a, Vec<u8>, (u64, Option<Vec<u8>>)>,
}
impl<'a> BTreeMemtableIterator<'a> {
    pub fn new(
        first_entry: Option<Entry<'a>>,
        last_entry: Option<Entry<'a>>,
        iterator: Iter<'a, Vec<u8>, (u64, Option<Vec<u8>>)>,
    ) -> Self {
        Self {
            first_entry,
            last_entry,
            iterator,
        }
    }
}
impl<'a> Iterator for BTreeMemtableIterator<'a> {
    type Item = Entry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        BTreeMemtable::get_entry_from_btree_entry(self.iterator.next())
    }
}
impl MemtableIterator for BTreeMemtableIterator<'_> {
    fn get_first_entry(&self) -> Option<Entry<'_>> {
        self.first_entry.clone()
    }
    fn get_last_entry(&self) -> Option<Entry<'_>> {
        self.last_entry.clone()
    }
}
