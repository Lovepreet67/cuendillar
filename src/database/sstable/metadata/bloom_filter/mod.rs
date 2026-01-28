use std::fmt::Write;

pub mod default_bloom_filter;
#[cfg(test)]
mod tests;
pub trait BloomFilter: Send + Sync {
    fn add(&mut self, key: &[u8]);
    fn check(&self, key: &[u8]) -> bool;
    fn serialize(&self, buf: &mut dyn Write);
}
