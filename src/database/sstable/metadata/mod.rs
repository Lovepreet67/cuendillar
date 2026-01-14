use crate::database::sstable::metadata::{
    bloom_filter::default_bloom_filter::DefaultBloomFilter, index::default_index::DefaultIndex,
};

pub mod bloom_filter;
pub mod index;

pub struct SSTMetadata {
    pub id: uuid::Uuid,
    pub bloom: DefaultBloomFilter,
    pub index: DefaultIndex,
}

impl SSTMetadata {
    pub fn new(id: uuid::Uuid, bloom: DefaultBloomFilter, index: DefaultIndex) -> Self {
        Self { id, bloom, index }
    }
}
