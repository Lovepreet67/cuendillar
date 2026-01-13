use std::{
    fs::{File, create_dir_all},
    io::Write,
    path::PathBuf,
};

use crate::database::{
    OwnedEntry,
    memtable::Memtable,
    sstable::{
        SSTable,
        bloom_filter::{BloomFilter, default_bloom_filter::DefaultBloomFilter},
        errors::SSTableError,
    },
};

pub struct DefaultSSTable {
    root_dir: PathBuf,
    metadata_file: File,
    blooms: Vec<DefaultBloomFilter>,
}

impl DefaultSSTable {
    pub fn new(root_dir: PathBuf) -> Result<Self, std::io::Error> {
        // first we will check if the parrent dir exists or not
        if !root_dir.exists() {
            create_dir_all(&root_dir)?;
        }
        let metadata_path = root_dir.join("metadata.sst");
        let metadata_file = File::options()
            .create(true)
            .read(true)
            .write(true)
            .open(metadata_path)?;
        Ok(Self {
            blooms: Vec::default(),
            root_dir,
            metadata_file,
        })
    }
}
impl SSTable for DefaultSSTable {
    fn push_memtable(&mut self, mt: &impl Memtable) -> Result<(), SSTableError> {
        let new_table_id = format!("{}", mt.get_id());
        let new_table_path = self.root_dir.join(&new_table_id);
        let mut writer = File::options()
            .append(true)
            .create_new(true)
            .open(new_table_path)?;
        self.metadata_file.write_all(new_table_id.as_bytes())?;
        self.metadata_file.write_all(b"\n")?;
        let mut bloom = DefaultBloomFilter::new(*mt.get_id(), 10000, 100);
        for i in mt.iter() {
            i.encode(&mut writer)?;
            bloom.add(i.get_key());
        }
        self.blooms.push(bloom);
        Ok(())
    }
    fn find(&self, id: &[u8]) -> Result<Option<OwnedEntry>, SSTableError> {
        // we will now use bloom filters and skip this metadata file call;
        for bloom in self.blooms.iter().rev() {
            // check if the key is in bloom filter
            if bloom.check(id) {
                let table_path = self.root_dir.join(&bloom.get_id().to_string());
                let mut reader = File::options().read(true).open(table_path)?;
                while let Ok(entry) = OwnedEntry::decode(&mut reader) {
                    if id == entry.get_id() {
                        return Ok(Some(entry));
                    }
                }
            }
        }
        return Ok(None);
    }
}
