use crate::database::config::bloom_config::BloomConfig;
use crate::database::sstable::metadata::bloom_filter::{
    default_bloom_filter::DefaultBloomFilter, tests::bloom_filter_test_insertion_and_check,
};

#[test]
fn default_bloom_filter_test() {
    let bf = DefaultBloomFilter::new(&&BloomConfig::get_test_config(), 3);
    bloom_filter_test_insertion_and_check(bf);
}
