use std::collections::BinaryHeap;

use crate::{
    OwnedEntry,
    database::{Entry, iterator::DatabaseIterator},
};

impl PartialEq for Box<dyn DatabaseIterator<Item = OwnedEntry>> {
    fn eq(&self, other: &Self) -> bool {
        match (self.peek(), other.peek()) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        }
    }
    fn ne(&self, other: &Self) -> bool {
        match (self.peek(), other.peek()) {
            (Some(x), Some(y)) => x != y,
            _ => true,
        }
    }
}

impl Eq for Box<dyn DatabaseIterator<Item = OwnedEntry>> {}

impl PartialOrd for Box<dyn DatabaseIterator<Item = OwnedEntry>> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Box<dyn DatabaseIterator<Item = OwnedEntry>> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.peek(), other.peek()) {
            (Some(_x), Some(_y)) => std::cmp::Ordering::Equal,
            (Some(_x), None) => std::cmp::Ordering::Greater,
            (None, Some(_y)) => std::cmp::Ordering::Less,
            (None, None) => std::cmp::Ordering::Equal,
        }
    }
}

pub struct MergedIterator {
    pub iterators: BinaryHeap<Box<dyn DatabaseIterator<Item = OwnedEntry>>>,
}

impl MergedIterator {
    pub fn new() -> Self {
        Self {
            iterators: BinaryHeap::new(),
        }
    }
    pub fn add_iterator(&mut self, iterator: Box<dyn DatabaseIterator<Item = OwnedEntry>>) {
        self.iterators.push(iterator);
    }
    pub fn peek(&mut self) /*-> OwnedEntry*/
    {
        // first peek all the iterators
    }
}
