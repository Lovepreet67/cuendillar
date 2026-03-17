use std::{
    clone::Clone,
    fmt::Debug,
    fs::File,
    io::{BufReader, Cursor, Read, Take, Write},
    os::unix::fs::FileExt,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use tracing::error;

use crate::database::{
    OwnedEntry,
    iterator::DatabaseIterator,
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
    pub data_block_size: u64,
    pub bloom_filter_size: u64,
    pub index_block_size: u64,
    pub key_range_block_size: u64,
}
impl SSTableFooter {
    pub fn new(
        data_block_size: u64,
        bloom_filter_size: u64,
        index_block_size: u64,
        key_range_block_size: u64,
    ) -> Self {
        Self {
            data_block_size,
            bloom_filter_size,
            index_block_size,
            key_range_block_size,
        }
    }
    pub fn deserialize(reader: &mut dyn Read) -> Result<Self, SSTableError> {
        let data_block_size = reader.read_u64::<BigEndian>()?;
        let bloom_filter_size = reader.read_u64::<BigEndian>()?;
        let index_block_size = reader.read_u64::<BigEndian>()?;
        let key_range_block_size = reader.read_u64::<BigEndian>()?;
        Ok(Self {
            data_block_size,
            bloom_filter_size,
            index_block_size,
            key_range_block_size,
        })
    }
    pub fn seriealize(&self, writer: &mut dyn Write) -> Result<u64, SSTableError> {
        writer.write_u64::<BigEndian>(self.data_block_size)?;
        writer.write_u64::<BigEndian>(self.bloom_filter_size)?;
        writer.write_u64::<BigEndian>(self.index_block_size)?;
        writer.write_u64::<BigEndian>(self.key_range_block_size)?;
        Ok(32)
    }
}

#[derive(Clone, Debug)]
pub struct SSTableKeyRange {
    pub first_key: Vec<u8>,
    pub last_key: Vec<u8>,
}

impl SSTableKeyRange {
    pub fn serialize(&self, writer: &mut dyn Write) -> Result<u64, SSTableError> {
        writer.write_u64::<BigEndian>(self.first_key.len() as u64)?;
        writer.write_all(&self.first_key)?;

        writer.write_u64::<BigEndian>(self.last_key.len() as u64)?;
        writer.write_all(&self.last_key)?;
        Ok(8 + self.first_key.len() as u64 + 8 + self.last_key.len() as u64)
    }
    pub fn deserialize(reader: &mut dyn Read) -> Result<Self, SSTableError> {
        let first_key_size = reader.read_u64::<BigEndian>()?;
        let mut first_key = vec![0; first_key_size as usize];
        reader.read_exact(&mut first_key)?;
        let last_key_size = reader.read_u64::<BigEndian>()?;
        let mut last_key = vec![0; last_key_size as usize];
        reader.read_exact(&mut last_key)?;
        Ok(Self {
            first_key,
            last_key,
        })
    }
}

pub struct SSTMetadata {
    pub id: uuid::Uuid,
    pub bloom: Arc<dyn BloomFilter>,
    pub index: Arc<dyn SSTIndex>,
    pub key_range: SSTableKeyRange,
    pub file: OnceLock<File>,
    pub file_path: PathBuf,
    pub footer: SSTableFooter,
}
impl Debug for SSTMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SSTMetadata")
            .field("id", &self.id)
            .field(
                "first_key",
                &String::from_utf8_lossy(&self.key_range.first_key),
            )
            .field(
                "last_key",
                &String::from_utf8_lossy(&self.key_range.last_key),
            )
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
            key_range: self.key_range.clone(),
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
            key_range: SSTableKeyRange {
                first_key,
                last_key,
            },
            file,
            file_path,
            footer,
        }
    }
    pub fn find(&self, key: &[u8]) -> Result<Option<OwnedEntry>, SSTableError> {
        if self.key_range.first_key.as_slice() <= key
            && self.key_range.last_key.as_slice() >= key
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
                if key == entry.get_key() {
                    return Ok(Some(entry));
                } else if entry.get_key() > key {
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
    pub fn iter(
        &self,
        start_key: Option<&[u8]>,
        end_key: Option<&[u8]>,
    ) -> Result<Box<dyn DatabaseIterator>, SSTableError> {
        let file = File::options().read(true).open(&self.file_path)?;
        // we can use Index to fetch the starting offset
        let start_offset = if let Some(start_key) = start_key {
            if let Some(offset) = self.index.get_offset(start_key) {
                offset.start
            } else {
                // fallback: start from beginning
                0
            }
        } else {
            0
        };
        let file_clone = file.try_clone()?; // important: We will clone the file so that other component will not be affected

        let mut data_reader = BufReader::new(file_clone.take(self.footer.data_block_size));

        // Seek to start offset manually
        if start_offset > 0 {
            data_reader.seek_relative(start_offset as i64)?;
        }

        // we will get the first and last entry from the sstable
        let last_entry = self.find(end_key.unwrap_or(&self.key_range.last_key))?;
        Ok(Box::new(SSTIterator::new(
            last_entry,
            data_reader,
            start_key,
            end_key,
        )))
    }
}

pub struct SSTIterator {
    first_entry: Option<OwnedEntry>,
    last_entry: Option<OwnedEntry>,
    reader: BufReader<Take<File>>,
    curr_entry: Option<OwnedEntry>,
    termination_key: Option<Vec<u8>>,
    terminated: bool,
}

impl SSTIterator {
    fn decode_entry(reader: &mut BufReader<Take<File>>) -> Option<OwnedEntry> {
        match OwnedEntry::decode(reader) {
            Ok(v) => Some(v),
            Err(e) => {
                error!("Error while reading entry during sstable iteration {:?}", e);
                return None;
            }
        }
    }
    pub fn new(
        last_entry: Option<OwnedEntry>,
        mut reader: BufReader<Take<File>>,
        start_key: Option<&[u8]>,
        termination_key: Option<&[u8]>,
    ) -> Self {
        let mut curr_entry = None;
        while let Some(entry) = Self::decode_entry(&mut reader) {
            if let Some(start) = start_key {
                if entry.get_key() < start {
                    continue;
                }
            }
            curr_entry = Some(entry);
            break;
        }
        Self {
            first_entry: curr_entry.clone(),
            last_entry: last_entry,
            reader,
            curr_entry,
            termination_key: termination_key.map(|key| key.into()),
            terminated: false,
        }
    }
}

impl DatabaseIterator for SSTIterator {
    fn peek(&self) -> Option<crate::database::Entry<'_>> {
        if self.terminated {
            return None;
        }
        self.curr_entry.as_ref().map(|e| e.into())
    }
    fn next_owned(&mut self) -> Option<OwnedEntry> {
        if self.terminated {
            return None;
        }
        let mut next_entry = Self::decode_entry(&mut self.reader);
        if let Some(entry) = &next_entry
            && self.termination_key.is_some()
        {
            if entry.get_key() > self.termination_key.as_ref().map(|e| e.as_slice()).unwrap() {
                self.terminated = true;
                next_entry = None;
            }
        }
        std::mem::replace(&mut self.curr_entry, next_entry)
    }
    fn first_entry(&self) -> Option<crate::database::Entry<'_>> {
        self.first_entry.as_ref().map(|e| e.into())
    }
    fn last_entry(&self) -> Option<crate::database::Entry<'_>> {
        self.last_entry.as_ref().map(|e| e.into())
    }
}
