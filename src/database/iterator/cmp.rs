use std::cmp::Ordering;

use crate::database::iterator::DatabaseIterator;

impl<'a> PartialEq for Box<dyn DatabaseIterator + 'a> {
    fn eq(&self, other: &Self) -> bool {
        self.peek() == other.peek()
    }
}

impl<'a> Eq for Box<dyn DatabaseIterator + 'a> {}

impl<'a> PartialOrd for Box<dyn DatabaseIterator + 'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for Box<dyn DatabaseIterator + 'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.peek(), other.peek()) {
            (Some(a), Some(b)) => {
                if a.get_key() < b.get_key() {
                    Ordering::Greater
                } else if a.get_key() > b.get_key() {
                    Ordering::Less
                }
                // if above two conditions are false then they two are equal
                else if a.get_seq_no() > b.get_seq_no() {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            } // reverse for min-heap behavior
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }
}
