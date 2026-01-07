use std::{io::Read, io::Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub mod db_engine;
mod errors;
mod memtable;
mod sstable;
mod wal;

#[derive(Debug, PartialEq, Clone)]
pub enum Entry<'a> {
    Tombstone { key: &'a [u8] },
    Row { key: &'a [u8], value: &'a [u8] },
}

impl Entry<'_> {
    pub fn encode(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        match self {
            crate::database::Entry::Row { key, value } => {
                writer.write_u64::<BigEndian>(key.len() as u64)?;
                writer.write(key)?;
                writer.write_u64::<BigEndian>(value.len() as u64)?;
                writer.write(value)?;
            }
            crate::database::Entry::Tombstone { key } => {
                writer.write_u64::<BigEndian>(key.len() as u64)?;
                writer.write(key)?;
                writer.write_u64::<BigEndian>(0 as u64)?;
            }
        }
        return Ok(());
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum OwnedEntry {
    Tombstone { key: Vec<u8> },
    Row { key: Vec<u8>, value: Vec<u8> },
}

impl OwnedEntry {
    pub fn decode(reader: &mut impl Read) -> Result<Self, std::io::Error> {
        let key_size = reader.read_u64::<BigEndian>()?;
        let mut key = vec![0u8; key_size as usize];
        reader.read_exact(&mut key)?;
        let val_size = reader.read_u64::<BigEndian>()?;
        if val_size == 0 {
            return Ok(OwnedEntry::Tombstone { key: key });
        }
        let mut value = vec![0u8; val_size as usize];
        reader.read_exact(&mut value)?;
        Ok(OwnedEntry::Row { key, value })
    }
}

impl<'a> From<&'a OwnedEntry> for Entry<'a> {
    fn from(value: &'a OwnedEntry) -> Self {
        match value {
            OwnedEntry::Row { key, value } => Entry::Row {
                key: key,
                value: value,
            },
            OwnedEntry::Tombstone { key } => Entry::Tombstone { key: key },
        }
    }
}
