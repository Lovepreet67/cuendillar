use std::{
    ffi::{OsStr, OsString},
    fs::{File, create_dir_all, read_dir, remove_file},
    io::{BufRead, BufReader, Seek, Write},
    path::PathBuf,
};

use byteorder::{BigEndian, WriteBytesExt};
use crc::Crc;

use crate::database::{
    Entry,
    wal::{MAGIC_NUMBER, WAL, errors::WALError, wal_entry::WALEntry},
};

pub struct DefaultWAL {
    active_log: Option<File>,
    wal_dir: PathBuf,
    wal_sync_group_size: u64,
    counter: u64,
    curr_offset: u64, // this will be used for lsn number
    crc_computer: Crc<u32>,
}
impl DefaultWAL {
    pub fn new(wal_dir: PathBuf, wal_sync_group_size: u64) -> Result<Self, WALError> {
        if !wal_dir.exists() {
            create_dir_all(&wal_dir)?;
        }

        let mut wal = Self {
            wal_dir,
            active_log: None,
            wal_sync_group_size,
            counter: 0,
            curr_offset: 0,
            crc_computer: Crc::<u32>::new(&crc::CRC_32_CKSUM),
        };
        let mut files = wal.get_all_files()?;
        let (current_offset, active_log_file) = if files.len() == 0 {
            let new_file_path = wal.wal_dir.join("0.wal");
            let new_file = File::options()
                .create_new(true)
                .append(true)
                .open(new_file_path)?;
            (0, new_file)
        } else {
            let (file_offset, file_path) = files.pop().unwrap(); // as we know file.len()>0
            let mut active_file = File::options()
                .create_new(true)
                .append(true)
                .open(&file_path)?;
            let curr_offset = active_file.seek(std::io::SeekFrom::Current(0))?;
            (file_offset + curr_offset, active_file)
        };
        wal.active_log = Some(active_log_file);
        wal.curr_offset = current_offset;
        Ok(wal)
    }
    pub fn rotate(&mut self) -> Result<(), WALError> {
        let new_log_file_id = format!("{}.wal", self.curr_offset);
        let new_log_file_path = self.wal_dir.join(&new_log_file_id);
        let new_log_file = File::options()
            .create_new(true)
            .append(true)
            .open(new_log_file_path)?;
        if let Some(active_log) = self.active_log.take() {
            drop(active_log);
        }
        new_log_file.sync_all()?;
        self.active_log = Some(new_log_file);
        Ok(())
    }
    fn get_all_files(&self) -> Result<Vec<(u64, PathBuf)>, WALError> {
        // we will check for the file and read its last entry
        let dir_enteries = read_dir(&self.wal_dir)?;
        let mut files = vec![];
        for dir_entry in dir_enteries {
            let dir_entry = dir_entry?;
            if dir_entry.path().is_dir() {
                continue;
            }
            let file_path = dir_entry.path();
            let file_offset =
                u64::from_str_radix(file_path.to_str().unwrap().split('.').next().unwrap(), 10)
                    .unwrap();
            files.push((file_offset, dir_entry.path()));
        }
        Ok(files)
    }
}
impl WAL for DefaultWAL {
    fn append_log(&mut self, entry: &Entry) -> Result<(), WALError> {
        if self.active_log.is_none() {
            self.rotate()?;
        }
        // we will compute crc and add lsn to file
        let mut payload = Vec::new();
        entry.encode(&mut payload)?;
        // now we will write the logs to file

        // we will create a buffer localy so that we can do write all
        let mut local_buff = Vec::new();
        local_buff.write_u64::<BigEndian>(self.curr_offset)?;
        local_buff.write_u32::<BigEndian>(self.crc_computer.checksum(&payload))?;
        local_buff.write_u64::<BigEndian>(payload.len() as u64)?;
        local_buff.write_all(&payload)?;
        local_buff.write_u64::<BigEndian>(MAGIC_NUMBER)?;
        // we will make this localbuff allign to 8
        while local_buff.len() % 8 != 0 {
            local_buff.write_u8(0)?;
        }
        assert!(self.active_log.is_some());
        let active_log = self.active_log.as_mut().unwrap();
        active_log.write_all(&local_buff)?;
        self.curr_offset += local_buff.len() as u64;
        self.counter += 1;
        if self.counter >= self.wal_sync_group_size {
            self.counter = 0;
            active_log.sync_data()?;
        }
        Ok(())
    }
    fn read(&mut self, offset: u64) -> Result<Vec<crate::database::OwnedEntry>, WALError> {
        let files = self.get_all_files()?;
        if files.len() == 0 {
            return Ok(vec![]);
        }
        let mut file_index = 0;
        while file_index < files.len() && offset > files[file_index].0 {
            file_index += 1;
        }
        // decrement the file to the next
        file_index -= 1;

        let mut reader = File::options().read(true).create(false).open("./test")?;
        let mut tor = vec![];
        while let Ok(entry) = WALEntry::decode(&mut reader) {
            // we will check for crc mismatch
        }
        Ok(tor)
    }
    fn flush_wal(&mut self, offset: u64) -> Result<(), WALError> {
        // we will delete all the files which contain offset less than the offset provided and remove them
        // read dir
        // find file with mentioned details
        // delete it

        return Ok(());
    }
}
