use crate::database::{
    config::{BloomConfig, variants::BloomVariant},
    sstable::metadata::bloom_filter::{BloomFilter, default_bloom_filter::DefaultBloomFilter},
};

pub fn build_bloom_filter(bloom_config: &BloomConfig) -> Box<dyn BloomFilter> {
    match bloom_config.variant {
        BloomVariant::Default => Box::new(DefaultBloomFilter::new(
            bloom_config.size,
            bloom_config.key_size,
        )),
    }
}
