use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

use crate::database::{
    config::bloom_config::{BloomConfig, BloomVariant},
    sstable::{
        errors::SSTableError,
        metadata::bloom_filter::{BloomFilter, default_bloom_filter::DefaultBloomFilter},
    },
};

pub struct BloomFactory;

impl BloomFactory {
    pub fn build_bloom_filter(bloom_config: &BloomConfig, table_size: u64) -> Box<dyn BloomFilter> {
        match bloom_config.variant {
            BloomVariant::Default => Box::new(DefaultBloomFilter::new(bloom_config, table_size)),
        }
    }
    pub fn deserialize_bloom_filter(
        reader: &mut dyn Read,
    ) -> Result<Box<dyn BloomFilter>, SSTableError> {
        // first we will read name
        let name_byte_len = reader.read_u16::<BigEndian>()?;
        let mut name_bytes = vec![0u8; name_byte_len as usize];
        reader.read_exact(&mut name_bytes)?;
        let name = std::str::from_utf8(&name_bytes)?;
        match name.into() {
            BloomVariant::Default => {
                let bloom_filter = DefaultBloomFilter::deserialize(reader)?;
                return Ok(bloom_filter);
            }
        }
    }
}
