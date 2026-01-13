use crate::database::sstable::bloom_filter::{
    default_bloom_filter::DefaultBloomFilter, tests::bloom_filter_test_insertion_and_check,
};

#[test]
fn default_bloom_filter_test() {
    let bf = DefaultBloomFilter::new(uuid::Uuid::new_v4(), 100, 10);
    bloom_filter_test_insertion_and_check(bf);
}
