use crate::{
    OwnedEntry,
    database::{Entry, iterator::DatabaseIterator},
};

use std::collections::BinaryHeap;

pub struct MergedIterator<'a> {
    iterators: BinaryHeap<Box<dyn DatabaseIterator + 'a>>,
    first_entry: Option<OwnedEntry>,
    last_entry: Option<OwnedEntry>,
}
impl<'a> MergedIterator<'a> {
    pub fn new() -> Self {
        Self {
            iterators: BinaryHeap::new(),
            first_entry: None,
            last_entry: None,
        }
    }

    pub fn add_iterator(&mut self, iterator: Box<dyn DatabaseIterator + 'a>) {
        if iterator.peek().is_some() {
            // now we will upadate our first and last key to update this for merged iterator
            match (self.first_entry.take(), iterator.first_entry()) {
                (Some(existing), Some(new)) => {
                    // key will be changed only if the new key is smaller than existing first key
                    // or the new key is newer variant of existing key
                    if (existing.get_key() > new.get_key())
                        || (existing.get_key() == new.get_key()
                            && existing.get_seq_no() < new.get_seq_no())
                    {
                        self.first_entry = Some(new.into())
                    } else {
                        self.first_entry = Some(existing)
                    }
                }
                (None, Some(new)) => self.first_entry = Some(new.into()),
                _ => {
                    // This case will not happen so we will not do anything
                }
            }
            match (self.last_entry.take(), iterator.last_entry()) {
                (Some(existing), Some(new)) => {
                    // key will be changed only if the new key is greater than existing last key
                    // or the new key is newer variant of existing key
                    if (existing.get_key() < new.get_key())
                        || (existing.get_key() == new.get_key()
                            && existing.get_seq_no() < new.get_seq_no())
                    {
                        self.last_entry = Some(new.into())
                    } else {
                        self.last_entry = Some(existing)
                    }
                }
                (None, Some(new)) => self.first_entry = Some(new.into()),
                _ => {
                    // This case will not happen so we will not do anything
                }
            }
            self.iterators.push(iterator);
        }
    }
}
impl<'a> DatabaseIterator for MergedIterator<'a> {
    fn peek(&self) -> Option<Entry<'_>> {
        self.iterators.peek()?.peek()
    }
    fn next_owned(&mut self) -> Option<OwnedEntry> {
        let mut top = self.iterators.pop()?;

        let result = top.next_owned();
        if top.peek().is_some() {
            self.iterators.push(top);
        }
        // now we need to remove all entries which contain same keys
        while result.is_some()
            && let Some(x) = self.iterators.peek()
        {
            if let Some(x) = x.peek() {
                if x.get_key() == result.as_ref()?.get_key() {
                    let mut top = self.iterators.pop()?;
                    top.next_owned();
                    if top.peek().is_some() {
                        self.iterators.push(top);
                    }
                } else {
                    break;
                }
            }
        }
        result
    }

    fn first_entry(&self) -> Option<Entry<'_>> {
        self.first_entry.as_ref().map(|e| e.into())
    }
    fn last_entry(&self) -> Option<Entry<'_>> {
        self.last_entry.as_ref().map(|e| e.into())
    }
}
