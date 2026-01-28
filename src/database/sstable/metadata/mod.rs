use std::{
    clone::Clone,
    fmt::Debug,
    fs::File,
    io::{Cursor, Read},
    os::unix::fs::FileExt,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use crate::database::{
    OwnedEntry,
    sstable::{
        errors::SSTableError,
        metadata::{bloom_filter::BloomFilter, index::SSTIndex},
    },
};

pub mod bloom_filter;
pub mod index;

// SSTable Footer will be of fixed size
// 8+8+8 = 32 bytes
// this will help us to divide table and decode parts accordingly
#[derive(Clone, Copy, Debug)]
pub struct SSTableFooter {
    data_block_size: u64,
    bloom_filter_size: u64,
    index_block_size: u64,
}
impl SSTableFooter {
    pub fn new(data_block_size: u64, bloom_filter_size: u64, index_block_size: u64) -> Self {
        Self {
            data_block_size,
            bloom_filter_size,
            index_block_size,
        }
    }
}
pub struct SSTMetadata {
    pub id: uuid::Uuid,
    pub bloom: Arc<dyn BloomFilter>,
    pub index: Arc<dyn SSTIndex>,
    pub first_key: Vec<u8>,
    pub last_key: Vec<u8>,
    pub file: OnceLock<File>,
    pub file_path: PathBuf,
    pub footer: SSTableFooter,
}
impl Debug for SSTMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SSTMetadata")
            .field("id", &self.id)
            .field("first_key", &String::from_utf8_lossy(&self.first_key))
            .field("last_key", &String::from_utf8_lossy(&self.last_key))
            .field("footer", &self.footer)
            .finish()
    }
}

impl Clone for SSTMetadata {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            bloom: self.bloom.clone(),
            index: self.index.clone(),
            first_key: self.first_key.clone(),
            last_key: self.last_key.clone(),
            file: OnceLock::new(),
            file_path: self.file_path.clone(),
            footer: self.footer,
        }
    }
}

impl SSTMetadata {
    pub fn new(
        id: uuid::Uuid,
        bloom: Arc<dyn BloomFilter>,
        index: Arc<dyn SSTIndex>,
        first_key: Vec<u8>,
        last_key: Vec<u8>,
        file: OnceLock<File>,
        file_path: PathBuf,
        footer: SSTableFooter,
    ) -> Self {
        Self {
            id,
            bloom,
            index,
            first_key,
            last_key,
            file,
            file_path,
            footer,
        }
    }
    pub fn find(&self, key: &[u8]) -> Result<Option<OwnedEntry>, SSTableError> {
        if self.first_key.as_slice() <= key
            && self.last_key.as_slice() >= key
            && self.bloom.check(key)
        {
            let block_offset = if let Some(block_offset) = self.index.get_offset(key) {
                block_offset
            } else {
                return Ok(None);
            };
            // TODO: We may be doing an syscall if some other thread initialize oncecell between get and get_or_init;
            if self.file.get().is_none() {
                let file = File::options().read(true).open(&self.file_path)?;
                self.file.get_or_init(move || file);
            }
            let reader = self.file.get().expect("File Should always there");
            let mut buf = vec![0u8; (block_offset.end - block_offset.start) as usize];
            reader.read_exact_at(&mut buf, block_offset.start)?;
            let mut reader = Cursor::new(&buf);
            while let Ok(entry) = OwnedEntry::decode(&mut reader) {
                if key == entry.get_id() {
                    return Ok(Some(entry));
                } else if entry.get_id() > key {
                    break;
                }
            }
        }
        return Ok(None);
    }

    pub fn item_list(&self) -> Result<Vec<OwnedEntry>, SSTableError> {
        let reader = File::options().read(true).open(&self.file_path)?;
        // we will limit the reader to data block only
        let mut data_reader = reader.take(self.footer.data_block_size);
        let mut enteries = vec![];
        while let Ok(entry) = OwnedEntry::decode(&mut data_reader) {
            enteries.push(entry);
        }
        Ok(enteries)
    }
    pub fn get_size(&self) -> u64 {
        self.footer.data_block_size + self.footer.bloom_filter_size + self.footer.index_block_size
    }
}
