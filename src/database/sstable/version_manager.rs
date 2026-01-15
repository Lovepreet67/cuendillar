use std::{
    cell::OnceCell,
    fs::{File, create_dir_all},
    path::PathBuf,
};

use crate::database::{
    memtable::{Memtable, MemtableIterator},
    sstable::{
        errors::SSTableError,
        metadata::{
            SSTMetadata,
            bloom_filter::{BloomFilter, default_bloom_filter::DefaultBloomFilter},
            index::{SSTIndex, default_index::DefaultIndex},
        },
        version::Version,
    },
};

pub struct VersionManager {
    root_dir: PathBuf,
    versions: Vec<Version>,
}
const INDEX_BLOCK_MIN_BYTES: u64 = 400;

impl VersionManager {
    pub fn new(root_dir: PathBuf) -> Self {
        create_dir_all(&root_dir).unwrap();
        Self {
            root_dir,
            // we will insert version which doesn't contain any sstable
            versions: vec![Version::new(Vec::default())],
        }
    }
    pub fn get_latest_version(&self) -> &Version {
        assert!(self.versions.len() > 0);
        self.versions.last().unwrap()
    }
    /// This Function doesn't change anything it returns the new version which caller need to to add to version manager
    /// Calling push_version
    pub fn push_memtable(&self, mt: &impl Memtable) -> Result<Version, SSTableError> {
        assert!(mt.size() > 0);
        let new_table_id = format!("{}", mt.get_id());
        let new_table_path = self.root_dir.join(&new_table_id);
        let mut writer = File::options()
            .append(true)
            .create_new(true)
            .open(&new_table_path)?;
        let mut bloom = DefaultBloomFilter::new(10000, 100);
        let mut index = DefaultIndex::new();
        let mut bytes_encoded = 0;
        let mut byte_encoded_since_last_index = INDEX_BLOCK_MIN_BYTES;
        let mt_iter = mt.iter();
        let first_key = mt_iter
            .get_first_entry()
            .expect("Memtable to Be flushed should contain atleast one entry")
            .get_key()
            .into();
        let last_key = mt_iter.get_last_entry().unwrap().get_key().into();
        for i in mt_iter {
            // check if entry is eligible for entry
            if byte_encoded_since_last_index >= INDEX_BLOCK_MIN_BYTES {
                index.add_entry(i.get_key(), bytes_encoded);
                byte_encoded_since_last_index = 0;
            }
            let bytes_encoded_for_this_entry = i.encode(&mut writer)?;
            byte_encoded_since_last_index += bytes_encoded_for_this_entry;
            bytes_encoded += bytes_encoded_for_this_entry;
            bloom.add(i.get_key());
        }
        index.add_last_offset(bytes_encoded);
        let sst_meta = SSTMetadata::new(
            *mt.get_id(),
            bloom,
            index,
            first_key,
            last_key,
            OnceCell::new(),
            new_table_path,
        );
        let latest_version = if self.versions.len() > 0 {
            self.get_latest_version().clone()
        } else {
            Version::new(Vec::default())
        };
        // now we will update
        // we will insert this to the the L0 of the latest version
        Ok(latest_version.add_l0_table(sst_meta))
    }

    pub fn push_version(&mut self, v: Version) {
        self.versions.push(v);
    }
}
