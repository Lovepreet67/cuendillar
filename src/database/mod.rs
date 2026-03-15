use std::{
    fmt::Debug,
    io::{Read, Write},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub mod config;
pub mod db_engine;
mod errors;
pub mod factory;
pub mod iterator;
mod memtable;
mod sstable;
pub mod wal;

#[derive(PartialEq, Clone)]
pub enum Entry<'a> {
    Tombstone {
        seq_no: u64,
        key: &'a [u8],
    },
    Row {
        seq_no: u64,
        key: &'a [u8],
        value: &'a [u8],
    },
}
impl<'a> Entry<'a> {
    pub fn from_key_value(seq_no: u64, key: &'a [u8], value: &'a [u8]) -> Self {
        if value.is_empty() {
            return Self::Tombstone { seq_no, key };
        }
        return Self::Row { seq_no, key, value };
    }
}
impl Debug for Entry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Row { seq_no, key, value } => {
                f.debug_struct("Entry")
                    .field("seq_no", seq_no)
                    .field(
                        "key",
                        &String::from_utf8(key.to_vec()).unwrap_or(format!("{:?}", key)),
                    )
                    .field(
                        "value",
                        &String::from_utf8(value.to_vec()).unwrap_or(format!("{:?}", value)),
                    )
                    .finish()?;
            }
            Self::Tombstone { seq_no, key } => {
                f.debug_struct("Entry")
                    .field("seq_no", seq_no)
                    .field(
                        "key",
                        &String::from_utf8(key.to_vec()).unwrap_or(format!("{:?}", key)),
                    )
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
            crate::database::Entry::Row { seq_no, key, value } => {
                bytes_writen += 8;
                writer.write_u64::<BigEndian>(*seq_no)?;
                bytes_writen += 4;
                writer.write_u32::<BigEndian>(key.len() as u32)?;
                bytes_writen += key.len();
                writer.write(key)?;
                bytes_writen += 4;
                writer.write_u32::<BigEndian>(value.len() as u32)?;
                bytes_writen += value.len();
                writer.write(value)?;
            }
            crate::database::Entry::Tombstone { seq_no, key } => {
                bytes_writen += 8;
                writer.write_u64::<BigEndian>(*seq_no)?;
                bytes_writen += 4;
                writer.write_u32::<BigEndian>(key.len() as u32)?;
                bytes_writen += key.len();
                writer.write(key)?;
                bytes_writen += 4;
                writer.write_u32::<BigEndian>(0 as u32)?;
            }
        }
        return Ok(bytes_writen as u64);
    }
    pub fn get_key(&self) -> &[u8] {
        return match self {
            Self::Row {
                seq_no: _,
                key,
                value: _,
            } => &key,
            Self::Tombstone { seq_no: _, key } => &key,
        };
    }
    pub fn get_seq_no(&self) -> u64 {
        return match self {
            Self::Row {
                seq_no,
                key: _,
                value: _,
            } => *seq_no,
            Self::Tombstone { seq_no, key: _ } => *seq_no,
        };
    }
}

#[derive(PartialEq, Clone)]
pub enum OwnedEntry {
    Tombstone {
        seq_no: u64,
        key: Vec<u8>,
    },
    Row {
        seq_no: u64,
        key: Vec<u8>,
        value: Vec<u8>,
    },
}

impl OwnedEntry {
    pub fn get_key(&self) -> &[u8] {
        return match self {
            Self::Row {
                seq_no: _,
                key,
                value: _,
            } => &key,
            Self::Tombstone { seq_no: _, key } => &key,
        };
    }
    pub fn get_seq_no(&self) -> u64 {
        return match self {
            Self::Row {
                seq_no,
                key: _,
                value: _,
            } => *seq_no,
            Self::Tombstone { seq_no, key: _ } => *seq_no,
        };
    }
    pub fn encode(&self, writer: &mut impl Write) -> Result<u64, std::io::Error> {
        let mut bytes_writen = 0;
        match self {
            crate::database::OwnedEntry::Row { seq_no, key, value } => {
                bytes_writen += 8;
                writer.write_u64::<BigEndian>(*seq_no)?;
                bytes_writen += 4;
                writer.write_u32::<BigEndian>(key.len() as u32)?;
                bytes_writen += key.len();
                writer.write(key)?;
                bytes_writen += 4;
                writer.write_u32::<BigEndian>(value.len() as u32)?;
                bytes_writen += value.len();
                writer.write(value)?;
            }
            crate::database::OwnedEntry::Tombstone { seq_no, key } => {
                bytes_writen += 8;
                writer.write_u64::<BigEndian>(*seq_no)?;
                bytes_writen += 4;
                writer.write_u32::<BigEndian>(key.len() as u32)?;
                bytes_writen += key.len();
                writer.write(key)?;
                bytes_writen += 4;
                writer.write_u32::<BigEndian>(0 as u32)?;
            }
        }
        return Ok(bytes_writen as u64);
    }
    pub fn decode(reader: &mut impl Read) -> Result<Self, std::io::Error> {
        let seq_no = reader.read_u64::<BigEndian>()?;
        let key_size = reader.read_u32::<BigEndian>()?;
        let mut key = vec![0u8; key_size as usize];
        reader.read_exact(&mut key)?;
        let val_size = reader.read_u32::<BigEndian>()?;
        if val_size == 0 {
            return Ok(OwnedEntry::Tombstone { seq_no, key: key });
        }
        let mut value = vec![0u8; val_size as usize];
        reader.read_exact(&mut value)?;
        Ok(OwnedEntry::Row { seq_no, key, value })
    }
    pub fn is_equal(&self, e: &Self) -> bool {
        match (self, e) {
            (
                Self::Row {
                    seq_no: _,
                    key: k1,
                    value: v1,
                },
                Self::Row {
                    seq_no: _,
                    key: k2,
                    value: v2,
                },
            ) => {
                return k1 == k2 && v1 == v2;
            }
            (Self::Tombstone { seq_no: _, key: k1 }, Self::Tombstone { seq_no: _, key: k2 }) => {
                return k1 == k2;
            }

            _ => {
                return false;
            }
        }
    }
}

impl Debug for OwnedEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Row { seq_no, key, value } => {
                f.debug_struct("OwnedEntry")
                    .field("seq_no", seq_no)
                    .field(
                        "key",
                        &String::from_utf8(key.to_vec()).unwrap_or(format!("{:?}", key)),
                    )
                    .field(
                        "value",
                        &String::from_utf8(value.to_vec()).unwrap_or(format!("{:?}", value)),
                    )
                    .finish()?;
            }
            Self::Tombstone { seq_no, key } => {
                f.debug_struct("OwnedEntry")
                    .field("seq_no", seq_no)
                    .field(
                        "key",
                        &String::from_utf8(key.to_vec()).unwrap_or(format!("{:?}", key)),
                    )
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
            OwnedEntry::Row { seq_no, key, value } => Entry::Row {
                seq_no: *seq_no,
                key: key,
                value: value,
            },
            OwnedEntry::Tombstone { seq_no, key } => Entry::Tombstone {
                seq_no: *seq_no,
                key: key,
            },
        }
    }
}
impl From<Entry<'_>> for OwnedEntry {
    fn from(value: Entry<'_>) -> Self {
        match value {
            Entry::Row { seq_no, key, value } => Self::Row {
                seq_no,
                key: key.into(),
                value: value.into(),
            },
            Entry::Tombstone { seq_no, key } => Self::Tombstone {
                seq_no,
                key: key.into(),
            },
        }
    }
}
