use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

use crate::database::{
    config::{IndexConfig, variants::IndexVariant},
    sstable::{
        errors::SSTableError,
        metadata::index::{SSTIndex, default_index::DefaultIndex},
    },
};

pub struct IndexFactory;

impl IndexFactory {
    pub fn build_index(config: &IndexConfig) -> Box<dyn SSTIndex> {
        match config.variant {
            IndexVariant::Default => Box::new(DefaultIndex::new()),
        }
    }
    pub fn deserialize_index(reader: &mut dyn Read) -> Result<Box<dyn SSTIndex>, SSTableError> {
        // first we will read name
        let name_byte_len = reader.read_u16::<BigEndian>()?;
        let mut name_bytes = vec![0u8; name_byte_len as usize];
        reader.read_exact(&mut name_bytes)?;
        let name = std::str::from_utf8(&name_bytes)?;
        match name.into() {
            IndexVariant::Default => {
                let default_index = DefaultIndex::deserialize(reader)?;
                return Ok(default_index);
            }
        }
    }
}
