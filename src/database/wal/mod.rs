use std::{
    convert::Infallible,
    fs::File,
    io::{IsTerminal, Read},
    path::{Path, PathBuf},
};

use byteorder::{BigEndian, ReadBytesExt};
use crc::Crc;

use crate::database::{
    Entry, OwnedEntry,
    wal::{errors::WALError, wal_entry::WALEntry},
};

pub mod default_wal;
pub mod errors;
#[cfg(test)]
mod tests;
pub mod wal_entry;

pub const MAGIC_NUMBER: u64 = 0x123232;

pub trait WAL {
    fn append_log(&mut self, entry: &Entry<'_>) -> Result<(), WALError>;
    fn read(&mut self, offset: u64) -> Result<Vec<OwnedEntry>, WALError>;
    fn flush_wal(&mut self, offset: u64) -> Result<(), WALError>;
}

pub struct WALIterator {
    files: Vec<PathBuf>,
    active_file: File,
    error: Option<WALError>,
    checksum_algo: Crc<u32>,
}
impl WALIterator {
    pub fn new() {}

    fn handle_read_result(
        &mut self,
        res: Result<Infallible, std::io::Error>,
        file: &Path,
    ) -> Option<Result<Infallible, WALError>> {
        let result = match res {
            Ok(v) => Ok(v),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // clean EOF → end of WAL
                Err(WALError::UnexpectedEndOfFile(file.to_path_buf()))
            }
            Err(e) => Err(WALError::IOError(e.to_string())),
        };
        if let Err(e) = result.clone() {
            self.error = Some(e);
        }
        Some(result)
    }
}
impl Iterator for WALIterator {
    type Item = Result<OwnedEntry, WALError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.error.is_some() {
            return Some(Err(self.error.unwrap()));
        }
        // will read the active file first and assume that the files are in order to which we need to read
        let lsn = self.active_file.read_u64::<BigEndian>()?;
        let checksum = self.active_file.read_u32::<BigEndian>()?;
        let payload_len = self.active_file.read_u64::<BigEndian>()?;
        let mut payload = vec![0; payload_len as usize];
        self.active_file.read_exact(&mut payload)?;
        // now we will read the magic number and check
        let magic_number = self.active_file.read_u64::<BigEndian>()?;

        if magic_number != MAGIC_NUMBER {
            let error =
                WALError::CrrouptedMetadataFile("Error in the logging mechanishm".to_string());
            self.error = Some(error);
            return Some(Err(error));
        }
        let error = WALError::CrrouptedMetadataFile("Error in the logging mechanishm".to_string());
        self.error = Some(error);
        return Some(Err(error));
    }
}
