use crate::database::sstable::metadata::bloom_filter::BloomFilter;
mod default_bloom_filter;
fn bloom_filter_test_insertion_and_check(mut bf: impl BloomFilter) {
    // first we will insert he values and then check for it
    let keys = vec![b"key1", b"key2", b"key3"];
    for key in &keys {
        bf.add(*key);
    }
    for key in keys {
        assert!(bf.check(key));
    }
}
