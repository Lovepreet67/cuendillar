use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

use crate::database::{OwnedEntry, wal::errors::WALError};

#[derive(Default)]
pub struct WALEntry {
    pub lsn: u64,
    pub checksum: u16,
    pub payload: Vec<u8>,
}
impl WALEntry {
    pub fn to_entry(&mut self) -> Result<OwnedEntry, std::io::Error> {
        OwnedEntry::decode(&mut self.payload.as_slice())
    }
    pub fn decode(reader: &mut dyn Read) -> Result<WALEntry, WALError> {
        let lsn = reader.read_u64::<BigEndian>()?;
        let checksum = reader.read_u16::<BigEndian>()?;
        let payload_len = reader.read_u64::<BigEndian>()?;
        let mut payload = vec![0; payload_len as usize];

        reader.read_exact(&mut payload)?;
        Ok(WALEntry {
            lsn,
            checksum,
            payload,
        })
    }
}
