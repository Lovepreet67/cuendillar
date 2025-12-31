use std::{fs::remove_dir_all, path::PathBuf, str::FromStr};

use crate::database::sstable::{
    default_sstable::DefaultSSTable, tests::sstable_test_encoding_decoding,
};

#[test]
fn default_sstable_test_encoding_decoding() {
    let sstable_root = "./tables";
    let mut sst = DefaultSSTable::new(PathBuf::from_str(sstable_root).unwrap()).unwrap();
    sstable_test_encoding_decoding(&mut sst);
    remove_dir_all(sstable_root).unwrap();
}
