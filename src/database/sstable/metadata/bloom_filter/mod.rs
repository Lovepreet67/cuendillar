use std::io::Write;

use crate::database::sstable::errors::SSTableError;

pub mod bloom_factory;
pub mod default_bloom_filter;
#[cfg(test)]
mod tests;
pub trait BloomFilter: Send + Sync {
    fn get_name(&self) -> &str;
    fn add(&mut self, key: &[u8]);
    fn check(&self, key: &[u8]) -> bool;
    fn serialize(&self, buf: &mut dyn Write) -> Result<u64, SSTableError>;
}
