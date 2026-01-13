use std::{fmt::Write, io::Read};

pub mod default_bloom_filter;
#[cfg(test)]
mod tests;
pub trait BloomFilter {
    fn get_id(&self) -> uuid::Uuid;
    fn add(&mut self, key: &[u8]);
    fn check(&self, key: &[u8]) -> bool;
    fn serialize(&self, buf: impl Write);
    fn deserialize(reader: impl Read) -> Self;
}
