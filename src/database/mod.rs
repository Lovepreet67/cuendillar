use std::{
    fmt::Debug,
    io::{Read, Write},
    path::PathBuf,
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub mod db_engine;
mod errors;
mod memtable;
mod sstable;
mod wal;

#[derive(PartialEq, Clone)]
pub enum Entry<'a> {
    Tombstone { key: &'a [u8] },
    Row { key: &'a [u8], value: &'a [u8] },
}
impl Debug for Entry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Row { key, value } => {
                f.debug_struct("Entry")
                    .field("key", &String::from_utf8(key.to_vec()).unwrap())
                    .field("value", &String::from_utf8(value.to_vec()).unwrap())
                    .finish()?;
            }
            Self::Tombstone { key } => {
                f.debug_struct("Entry")
                    .field("key", &String::from_utf8(key.to_vec()).unwrap())
                    .field("value", &"None")
                    .finish()?;
            }
        }
        Ok(())
    }
}

impl Entry<'_> {
    pub fn encode(&self, writer: &mut impl Write) -> Result<u64, std::io::Error> {
        let mut bytes_writen = 0;
        match self {
            crate::database::Entry::Row { key, value } => {
                bytes_writen += 8;
                writer.write_u64::<BigEndian>(key.len() as u64)?;
                bytes_writen += key.len();
                writer.write(key)?;
                bytes_writen += 8;
                writer.write_u64::<BigEndian>(value.len() as u64)?;
                bytes_writen += value.len();
                writer.write(value)?;
            }
            crate::database::Entry::Tombstone { key } => {
                bytes_writen += 8;
                writer.write_u64::<BigEndian>(key.len() as u64)?;
                bytes_writen += key.len();
                writer.write(key)?;
                bytes_writen += 8;
                writer.write_u64::<BigEndian>(0 as u64)?;
            }
        }
        return Ok(bytes_writen as u64);
    }
    pub fn get_key(&self) -> &[u8] {
        return match self {
            Self::Row { key, value: _ } => &key,
            Self::Tombstone { key } => &key,
        };
    }
}

#[derive(PartialEq, Clone)]
pub enum OwnedEntry {
    Tombstone { key: Vec<u8> },
    Row { key: Vec<u8>, value: Vec<u8> },
}

impl OwnedEntry {
    pub fn get_id(&self) -> &[u8] {
        return match self {
            Self::Row { key, value: _ } => &key,
            Self::Tombstone { key } => &key,
        };
    }
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

impl Debug for OwnedEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Row { key, value } => {
                f.debug_struct("OwnedEntry")
                    .field("key", &String::from_utf8(key.to_vec()).unwrap())
                    .field("value", &String::from_utf8(value.to_vec()).unwrap())
                    .finish()?;
            }
            Self::Tombstone { key } => {
                f.debug_struct("OwnedEntry")
                    .field("key", &String::from_utf8(key.to_vec()).unwrap())
                    .field("value", &"None")
                    .finish()?;
            }
        }
        Ok(())
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
impl From<Entry<'_>> for OwnedEntry {
    fn from(value: Entry<'_>) -> Self {
        match value {
            Entry::Row { key, value } => Self::Row {
                key: key.into(),
                value: value.into(),
            },
            Entry::Tombstone { key } => Self::Tombstone { key: key.into() },
        }
    }
}
