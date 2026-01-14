use crate::database::sstable::metadata::bloom_filter::{
    default_bloom_filter::DefaultBloomFilter, tests::bloom_filter_test_insertion_and_check,
};

#[test]
fn default_bloom_filter_test() {
    let bf = DefaultBloomFilter::new(100, 10);
    bloom_filter_test_insertion_and_check(bf);
}
