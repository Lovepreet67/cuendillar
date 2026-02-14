use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use murmur3::murmur3_32;

use crate::database::{
    config::bloom_config::{BloomConfig, BloomVariant},
    sstable::{errors::SSTableError, metadata::bloom_filter::BloomFilter},
};

#[derive(Clone, Debug)]
pub struct DefaultBloomFilter {
    config: BloomConfig,
    bloom: Vec<u64>,
}

impl DefaultBloomFilter {
    pub fn new(config: &BloomConfig) -> Self {
        assert_eq!(config.variant, BloomVariant::Default);
        Self {
            config: config.clone(),
            bloom: vec![0; ((config.size + 63) / 64) as usize],
        }
    }
    fn bloom_size(&self) -> u32 {
        self.bloom.len() as u32 * 64 // as single index is containing the 64 bits
    }
    pub fn deserialize(reader: &mut dyn Read) -> Result<Box<Self>, SSTableError> {
        let bits_per_key = reader.read_u32::<BigEndian>()?;
        let bloom_size = (reader.read_u32::<BigEndian>()? / 64) as usize; // size written is in bits
        let mut bloom = vec![0; bloom_size as usize];
        for i in 0..bloom_size {
            bloom[i] = reader.read_u64::<BigEndian>()?;
        }
        Ok(Box::new(Self {
            config: BloomConfig {
                variant: crate::database::config::bloom_config::BloomVariant::Default,
                size: bloom_size,
                bits_per_key: bits_per_key as usize,
            },
            bloom,
        }))
    }
}

impl BloomFilter for DefaultBloomFilter {
    fn get_name(&self) -> &str {
        "default"
    }
    fn add(&mut self, key: &[u8]) {
        let mut x = key;
        let h1 = murmur3_32(&mut x, 0).unwrap();
        let delta = (h1 >> 17) | (h1 << 15);
        for i in 0..self.config.bits_per_key as u32 {
            let hi = h1.wrapping_add(i.wrapping_mul(delta)) % self.bloom_size();

            // now we will set a bit in the vector
            let index = (hi / 64) as usize;
            let bit_index = hi % 64;
            self.bloom[index] = self.bloom[index] | 1 << bit_index;
        }
    }
    fn check(&self, key: &[u8]) -> bool {
        let mut x = key;
        let h1 = murmur3_32(&mut x, 0).unwrap();
        let delta = (h1 >> 17) | (h1 << 15);
        for i in 0..self.config.bits_per_key as u32 {
            let hi = h1.wrapping_add(i.wrapping_mul(delta)) % self.bloom_size();
            // now we will set a bit in the vector
            let index = (hi / 64) as usize;
            let bit_index = hi % 64;
            if (self.bloom[index] & 1 << bit_index) == 0 {
                return false;
            }
        }
        return true;
    }
    fn serialize(&self, buf: &mut dyn std::io::Write) -> Result<u64, SSTableError> {
        // serialization of bloom filter
        let mut bytes_written = 0;
        buf.write_u32::<BigEndian>(self.config.bits_per_key as u32)?;
        bytes_written += 4;
        buf.write_u32::<BigEndian>(self.bloom_size())?;
        bytes_written += 4;
        for item in &self.bloom {
            buf.write_u64::<BigEndian>(*item)?;
            bytes_written += 8;
        }
        Ok(bytes_written)
    }
}
