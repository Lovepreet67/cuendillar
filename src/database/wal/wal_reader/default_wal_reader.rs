use std::{fs::File, io::BufReader, path::PathBuf};

use crate::database::wal::{wal_entry::WALEntry, wal_reader::WALReader};

pub struct DefaultWALReader {
    wal_dir: PathBuf,
    log_file_id: String,
}
impl DefaultWALReader {}

impl WALReader for DefaultWALReader {
    fn read(&mut self) -> Result<WALEntry, crate::database::wal::errors::WALReaderError> {
        let log_file_path = self.wal_dir.join(&self.log_file_id);
        let f = File::options()
            .read(true)
            .create(false)
            .open(log_file_path)?;
        Ok(WALEntry::default())
    }
}
