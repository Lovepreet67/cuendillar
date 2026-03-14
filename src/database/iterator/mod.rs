pub mod merged_iterator;
pub trait DatabaseIterator: Iterator {
    fn range(&self) -> Option<(&'static [u8], &'static [u8])>;
    fn peek(&self) -> Option<&[u8]>;
}
