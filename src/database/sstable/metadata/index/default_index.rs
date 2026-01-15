use std::{
    io::{Read, Write},
    ops::Range,
};

use crate::database::sstable::{errors::SSTableError, metadata::index::SSTIndex};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

#[derive(Clone)]
pub struct DefaultIndexEntry {
    key: Vec<u8>,
    offset: u64,
}
impl DefaultIndexEntry {
    pub fn serialize(&self, writer: &mut dyn Write) -> Result<u64, SSTableError> {
        let mut bytes_written: u64 = 0;
        bytes_written += 4;
        writer.write_u32::<BigEndian>(self.key.len() as u32)?;
        bytes_written += self.key.len() as u64;
        writer.write_all(&self.key)?;
        bytes_written += 8;
        writer.write_u64::<BigEndian>(self.offset)?;
        Ok(bytes_written)
    }
    pub fn deserizlize(reader: &mut dyn Read) -> Result<Self, SSTableError> {
        let key_size = reader.read_u32::<BigEndian>()?;
        let mut key = vec![0u8; key_size as usize];
        reader.read_exact(&mut key)?;
        let offset = reader.read_u64::<BigEndian>()?;
        Ok(Self { key, offset })
    }
}

#[derive(Default, Clone)]
pub struct DefaultIndex {
    entries: Vec<DefaultIndexEntry>,
    last_offset: u64,
}
impl DefaultIndex {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            last_offset: 0,
        }
    }
}
impl SSTIndex for DefaultIndex {
    /// This function assumes that this entery is > that enteries added previously
    fn add_entry(&mut self, key: &[u8], offset: u64) {
        self.entries.push(DefaultIndexEntry {
            key: key.into(),
            offset,
        });
    }
    fn add_last_offset(&mut self, last_offset: u64) {
        self.last_offset = last_offset;
    }
    fn get_offset(&self, key: &[u8]) -> Option<Range<u64>> {
        // we will use binary search here
        // we need both starting offset of  keys <= current key and starting offset to next key> current key -1

        let idx = match self
            .entries
            .binary_search_by(|entry| entry.key.as_slice().cmp(key))
        {
            Ok(i) => i,
            Err(0) => return None,
            Err(e) => e - 1, // e is where the key can be inserted mantaing the sorted order so e-1 will be key < curr_ley
        };
        // now we a index of key <= curr_key and we will find the index ending index it will be offset of next block or ending offset
        let block_end_offset = if idx == self.entries.len() - 1 {
            self.last_offset
        } else {
            self.entries[idx + 1].offset
        };

        Some(Range {
            start: self.entries[idx].offset,
            end: block_end_offset,
        })
    }
    fn serialize(&self, writer: &mut dyn std::io::Write) -> Result<u64, SSTableError> {
        // serialize the index for storage in the sstable

        let mut bytes_writen: u64 = 0;

        // first we will write the length of enteries as u64
        bytes_writen += 8;
        writer.write_u64::<BigEndian>(self.entries.len() as u64)?;
        // then we write each entery
        for i in &self.entries {
            bytes_writen += i.serialize(writer)?;
        }
        bytes_writen += 8;
        writer.write_u64::<BigEndian>(self.last_offset)?;
        Ok(bytes_writen)
    }
    fn deserialize(reader: &mut dyn std::io::Read) -> Result<Box<Self>, SSTableError> {
        // deserialize this

        // first we will fetch number of enteris
        let num_enteries = reader.read_u64::<BigEndian>()?;
        // then we will deserialize these enteries one by one
        let mut entries = Vec::with_capacity(num_enteries as usize);
        for _i in 0..num_enteries {
            entries.push(DefaultIndexEntry::deserizlize(reader)?);
        }
        Ok(Box::new(Self {
            entries,
            last_offset: reader.read_u64::<BigEndian>()?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::u64;

    use crate::database::sstable::metadata::index::{
        default_index::DefaultIndex,
        tests::{sst_index_test_add_and_get, sst_index_test_persistant},
    };

    #[test]
    fn default_sst_index_test_add_and_get() {
        let index = DefaultIndex {
            entries: vec![],
            last_offset: u64::MAX,
        };
        sst_index_test_add_and_get(index);
    }

    #[test]
    fn default_sst_test_persistance_test() {
        let index = DefaultIndex {
            entries: vec![],
            last_offset: u64::MAX,
        };
        sst_index_test_persistant::<DefaultIndex>(index);
    }
}
