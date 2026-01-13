use bit_set::BitSet;
use murmur3::murmur3_32;

use crate::database::sstable::bloom_filter::BloomFilter;

pub struct DefaultBloomFilter {
    id: uuid::Uuid,
    size: u32,
    bits_per_key: u32,
    bloom: BitSet,
}

impl DefaultBloomFilter {
    pub fn new(id: uuid::Uuid, size: u32, bits_per_key: u32) -> Self {
        Self {
            id,
            size,
            bits_per_key,
            bloom: BitSet::with_capacity(size as usize),
        }
    }
}

impl BloomFilter for DefaultBloomFilter {
    fn get_id(&self) -> uuid::Uuid {
        self.id
    }
    fn add(&mut self, key: &[u8]) {
        let mut x = key;
        let h1 = murmur3_32(&mut x, 0).unwrap();
        let delta = (h1 >> 17) | (h1 << 15);
        for i in 0..self.bits_per_key {
            let hi = h1.wrapping_add(i.wrapping_mul(delta)) % self.size;
            self.bloom.insert(hi as usize);
        }
    }
    fn check(&self, key: &[u8]) -> bool {
        let mut x = key;
        let h1 = murmur3_32(&mut x, 0).unwrap();
        let delta = (h1 >> 17) | (h1 << 15);
        for i in 0..self.bits_per_key {
            let hi = h1.wrapping_add(i.wrapping_mul(delta)) % self.size;
            if !self.bloom.contains(hi as usize) {
                return false;
            }
        }
        return true;
    }
    fn serialize(&self, buf: impl std::fmt::Write) {}
    fn deserialize(reader: impl std::io::Read) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            size: 0,
            bits_per_key: 5,
            bloom: BitSet::default(),
        }
    }
}
