use std::{
    fs::{File, create_dir_all, remove_file},
    io::{BufRead, BufReader, Seek, Write},
    path::PathBuf,
};

use crate::database::{
    OwnedEntry,
    wal::{WAL, errors::WALError, wal_entry::WALEntry},
};

pub struct DefaultWAL {
    active_log: Option<File>,
    wal_dir: PathBuf,
    metadata_file: File,
}
impl WAL for DefaultWAL {
    fn new(wal_dir: PathBuf) -> Result<Self, WALError> {
        if !wal_dir.exists() {
            create_dir_all(&wal_dir)?;
        }
        let metadata_file_path = wal_dir.join("metadata.wal");
        let f = File::options()
            .create(true)
            .write(true)
            .append(true)
            .read(true)
            .open(metadata_file_path)?;
        Ok(Self {
            wal_dir,
            metadata_file: f,
            active_log: None,
        })
    }
    fn rotate(&mut self, id: Option<uuid::Uuid>) -> Result<(), WALError> {
        let id = id.unwrap_or_else(|| uuid::Uuid::new_v4());
        let new_log_file_id = format!("{}.wal", id);
        let new_log_file_path = self.wal_dir.join(&new_log_file_id);
        let new_log_file = File::options()
            .create_new(true)
            .append(true)
            .open(new_log_file_path)?;
        if let Some(active_log) = self.active_log.take() {
            drop(active_log);
        }
        self.metadata_file.write_all(id.to_string().as_bytes())?;
        self.metadata_file.write_all("\n".as_bytes())?;
        self.active_log = Some(new_log_file);
        Ok(())
    }
    fn append_log(
        &mut self,
        entry: WALEntry,
    ) -> Result<(), crate::database::wal::errors::WALError> {
        if self.active_log.is_none() {
            self.rotate(None)?;
        }
        assert!(self.active_log.is_some());
        let active_log = self.active_log.as_mut().unwrap();
        active_log.write_all(entry.payload.as_slice())?;
        active_log.sync_data()?;
        Ok(())
    }
    fn read(&mut self, log_id: &uuid::Uuid) -> Result<Vec<crate::database::OwnedEntry>, WALError> {
        let log_file_id = format!("{}.wal", log_id);
        let log_file_path = self.wal_dir.join(&log_file_id);
        let mut reader = File::options()
            .read(true)
            .create(false)
            .open(log_file_path)?;
        let mut tor = vec![];
        while let Ok(entry) = OwnedEntry::decode(&mut reader) {
            tor.push(entry);
        }
        Ok(tor)
    }
    fn get_wals(&mut self) -> Result<Vec<uuid::Uuid>, WALError> {
        let current_cursor_possition = self.metadata_file.seek(std::io::SeekFrom::Current(0))?;
        self.metadata_file.seek(std::io::SeekFrom::Start(0))?;
        let buff_reader = BufReader::new(&mut self.metadata_file);
        let mut log_ids = vec![];
        for log_file_id in buff_reader.lines() {
            let log_file_id = log_file_id?;
            log_ids.push(uuid::Uuid::parse_str(&log_file_id)?);
        }
        self.metadata_file
            .seek(std::io::SeekFrom::Start(current_cursor_possition))?;
        Ok(log_ids)
    }
    fn flush_wal(&mut self, id: uuid::Uuid) -> Result<(), WALError> {
        self.metadata_file.write_all(id.to_string().as_bytes())?;
        self.metadata_file.write_all(" FLUSH\n".as_bytes())?;
        let log_file_id = self.wal_dir.join(format!("{}.wal", id));
        remove_file(log_file_id)?;
        return Ok(());
    }
}
