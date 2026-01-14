use std::{
    io::{Read, Write},
    ops::Range,
};

use crate::database::sstable::errors::SSTableError;

pub mod default_index;

#[cfg(test)]
mod tests;

pub trait SSTIndex {
    fn get_offset(&self, key: &[u8]) -> Option<Range<u64>>;
    fn add_entry(&mut self, key: &[u8], offset: u64);
    fn add_last_offset(&mut self, last_offset: u64);
    fn serialize(&self, writer: &mut dyn Write) -> Result<u64, SSTableError>;
    fn deserialize(reader: &mut dyn Read) -> Result<Box<Self>, SSTableError>;
}
