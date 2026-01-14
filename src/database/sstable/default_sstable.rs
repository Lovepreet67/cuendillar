use std::{
    fs::{File, create_dir_all},
    io::{Cursor, Read, Seek, Write},
    path::PathBuf,
};

use crate::database::{
    OwnedEntry,
    memtable::Memtable,
    sstable::{
        SSTable,
        errors::SSTableError,
        metadata::{
            SSTMetadata,
            bloom_filter::{BloomFilter, default_bloom_filter::DefaultBloomFilter},
            index::{SSTIndex, default_index::DefaultIndex},
        },
    },
};

const INDEX_BLOCK_MIN_BYTES: u64 = 400;

pub struct DefaultSSTable {
    root_dir: PathBuf,
    metadata_file: File,
    metadata: Vec<SSTMetadata>,
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
            metadata: Vec::default(),
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
        let mut bloom = DefaultBloomFilter::new(10000, 100);
        let mut index = DefaultIndex::new();
        let mut bytes_encoded = 0;
        let mut byte_encoded_since_last_index = INDEX_BLOCK_MIN_BYTES;
        for i in mt.iter() {
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
        self.metadata
            .push(SSTMetadata::new(*mt.get_id(), bloom, index));
        Ok(())
    }
    fn find(&self, key: &[u8]) -> Result<Option<OwnedEntry>, SSTableError> {
        // we will now use bloom filters and skip this metadata file call;
        for metatdata in self.metadata.iter().rev() {
            // check if the key is in bloom filter
            if metatdata.bloom.check(key) {
                let block_offset = if let Some(block_offset) = metatdata.index.get_offset(key) {
                    block_offset
                } else {
                    continue;
                };
                let table_path = self.root_dir.join(&metatdata.id.to_string());
                let mut reader = File::options().read(true).open(table_path)?;
                reader.seek(std::io::SeekFrom::Start(block_offset.start))?;
                let mut buf = vec![0u8; (block_offset.end - block_offset.start) as usize];
                reader.read_exact(&mut buf)?;
                drop(reader);
                let mut reader = Cursor::new(&buf);
                while let Ok(entry) = OwnedEntry::decode(&mut reader) {
                    if key == entry.get_id() {
                        return Ok(Some(entry));
                    } else if entry.get_id() > key {
                        break;
                    }
                }
            }
        }
        return Ok(None);
    }
}
