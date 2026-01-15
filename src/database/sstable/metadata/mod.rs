use std::{cell::OnceCell, fs::File, io::Cursor, os::unix::fs::FileExt, path::PathBuf};

use crate::database::{
    OwnedEntry,
    sstable::{
        errors::SSTableError,
        metadata::{
            bloom_filter::{BloomFilter, default_bloom_filter::DefaultBloomFilter},
            index::{SSTIndex, default_index::DefaultIndex},
        },
    },
};

pub mod bloom_filter;
pub mod index;
pub struct SSTMetadata {
    pub id: uuid::Uuid,
    pub bloom: DefaultBloomFilter,
    pub index: DefaultIndex,
    pub first_key: Vec<u8>,
    pub last_key: Vec<u8>,
    pub file: OnceCell<File>,
    pub file_path: PathBuf,
}

impl Clone for SSTMetadata {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            bloom: self.bloom.clone(),
            index: self.index.clone(),
            first_key: self.first_key.clone(),
            last_key: self.last_key.clone(),
            file: OnceCell::new(),
            file_path: self.file_path.clone(),
        }
    }
}

impl SSTMetadata {
    pub fn new(
        id: uuid::Uuid,
        bloom: DefaultBloomFilter,
        index: DefaultIndex,
        first_key: Vec<u8>,
        last_key: Vec<u8>,
        file: OnceCell<File>,
        file_path: PathBuf,
    ) -> Self {
        Self {
            id,
            bloom,
            index,
            first_key,
            last_key,
            file,
            file_path,
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
}
