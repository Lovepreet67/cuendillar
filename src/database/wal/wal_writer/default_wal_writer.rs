use std::{
    fs::{File, create_dir_all},
    io::Write,
    path::PathBuf,
};

use crate::database::wal::{wal_entry::WALEntry, wal_writer::WALWriter};

pub struct DefaultWALWriter {
    active_log: Option<File>,
    wal_dir: PathBuf,
    metadata_file: File,
}
impl WALWriter for DefaultWALWriter {
    fn new(wal_dir: PathBuf) -> Result<Self, std::io::Error> {
        if !wal_dir.exists() {
            create_dir_all(&wal_dir)?;
        }
        let metadata_file_path = wal_dir.join("metadata.wal");
        let f = File::options()
            .create(true)
            .write(true)
            .append(true)
            .open(metadata_file_path)?;
        Ok(Self {
            wal_dir,
            metadata_file: f,
            active_log: None,
        })
    }
    fn rotate(&mut self, id: Option<uuid::Uuid>) -> Result<(), std::io::Error> {
        let new_log_file_id = format!("{}.wal", id.unwrap_or_else(|| uuid::Uuid::new_v4()));
        let new_log_file_path = self.wal_dir.join(&new_log_file_id);
        let new_log_file = File::options()
            .create_new(true)
            .append(true)
            .open(new_log_file_path)?;
        if let Some(active_log) = self.active_log.take() {
            drop(active_log);
        }
        self.metadata_file.write_all(new_log_file_id.as_bytes())?;
        self.metadata_file.write_all("\n".as_bytes())?;
        self.active_log = Some(new_log_file);
        Ok(())
    }
    fn append(
        &mut self,
        entry: WALEntry,
    ) -> Result<(), crate::database::wal::errors::WALWriteError> {
        assert!(self.active_log.is_some());
        let active_log = self.active_log.as_mut().unwrap();
        active_log.write_all(entry.payload.as_slice())?;
        active_log.sync_data()?;
        Ok(())
    }
}
