use crate::{OwnedEntry, database::Entry};

pub mod cmp;
pub mod merged_iterator;
#[cfg(test)]
mod test;

pub struct DatabaseIteratorAdapter<T> {
    inner: T,
}

impl<T> Iterator for DatabaseIteratorAdapter<T>
where
    T: DatabaseIterator,
{
    type Item = OwnedEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_owned()
    }
}
pub trait DatabaseIterator {
    fn peek(&self) -> Option<Entry<'_>>;
    fn next_owned(&mut self) -> Option<OwnedEntry>;
    fn first_entry(&self) -> Option<Entry<'_>>;
    fn last_entry(&self) -> Option<Entry<'_>>;
    fn as_iterator(self) -> DatabaseIteratorAdapter<Self>
    where
        Self: Sized,
    {
        DatabaseIteratorAdapter { inner: self }
    }
}
impl Iterator for Box<dyn DatabaseIterator + '_> {
    type Item = OwnedEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned()
    }
}
