use std::{
    fs::{File, create_dir_all, read_dir, remove_file},
    io::{Read, Seek, Write},
    path::PathBuf,
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crc::Crc;

use crate::database::wal::{MAGIC_NUMBER, MAX_PAYLOAD_LEN, WAL, WALIterator, errors::WALError};

fn eight_align_addition(value: u64) -> u64 {
    if value % 8 == 0 {
        return 0;
    }
    return 8 - value % 8;
}

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
                .create(true)
                .append(true)
                .open(new_file_path)?;
            (0, new_file)
        } else {
            let (file_offset, file_path) = files.pop().unwrap(); // as we know file.len()>0
            let mut active_file = File::options().create(true).append(true).open(&file_path)?;
            let curr_offset = active_file.seek(std::io::SeekFrom::Current(0))?;
            (file_offset + curr_offset, active_file)
        };
        wal.active_log = Some(active_log_file);
        wal.curr_offset = current_offset;
        Ok(wal)
    }
    pub fn rotate(&mut self) -> Result<(), WALError> {
        //TODO: curr offset is only in file not global offset

        let new_log_file_id = format!("{}.wal", self.curr_offset + 1);
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
        self.curr_offset = 0;
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
            let stem = file_path
                .as_path()
                .file_stem()
                .ok_or_else(|| WALError::InvalidFileName(file_path.clone()))?
                .to_str()
                .ok_or_else(|| WALError::InvalidFileName(file_path.clone()))?;
            let file_offset = stem
                .parse::<u64>()
                .map_err(|_e| WALError::InvalidFileName(file_path.clone()))?;
            files.push((file_offset, dir_entry.path()));
        }
        files.sort_by_key(|(offset, _)| *offset);
        Ok(files)
    }
}
impl WAL for DefaultWAL {
    fn append_log(&mut self, payload: &[u8]) -> Result<(), WALError> {
        if self.active_log.is_none() {
            self.rotate()?;
        }

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
    fn read(&mut self, offset: u64) -> Result<Box<dyn WALIterator>, WALError> {
        let files = self.get_all_files()?;
        if files.len() == 0 {
            return Ok(Box::new(DefaultWALIterator::new(
                vec![],
                None,
                self.crc_computer.clone(),
            )));
        }
        let mut file_index = 0;
        while file_index < files.len() && offset > files[file_index].0 {
            file_index += 1;
        }
        // file index always be greater than 0 as we will assume that the last file can contain the specified offset
        let offset_inside_file = offset - files[file_index - 1].0;
        let files_to_be_included = files
            .into_iter()
            .skip(file_index)
            .map(|item| item.1)
            .collect::<Vec<PathBuf>>();
        // now we will open the first file and move the pointer to the lsn to
        let mut active_file = File::options().read(true).open(&files_to_be_included[0])?;
        // we must check if the specified offset is in the file bounderies or not
        // TODO: check if the offset is legal for the specified file or not
        active_file.seek(std::io::SeekFrom::Start(offset_inside_file))?;
        return Ok(Box::new(DefaultWALIterator::new(
            files_to_be_included,
            Some(active_file),
            self.crc_computer.clone(),
        )));
    }
    fn flush_wal(&mut self, offset: u64) -> Result<(), WALError> {
        // we will delete all the files which contain offset less than the offset provided and remove them
        let files = self.get_all_files()?;
        let mut file_index = 0;
        while file_index < files.len() && offset > files[file_index].0 {
            file_index += 1;
        }
        // we will only delete files which are at index < file_index - 1
        if file_index <= 1 {
            return Ok(());
        }
        // NOTE: Files may be in use need to check in future
        for i in 0..file_index - 1 {
            remove_file(&files[i].1)?;
        }
        return Ok(());
    }
}

pub struct DefaultWALIterator {
    files: Vec<PathBuf>,
    active_file: Option<File>,
    error: Option<WALError>,
    checksum_algo: Crc<u32>,
    index: usize,
}
impl DefaultWALIterator {
    pub fn new(files: Vec<PathBuf>, active_file: Option<File>, checksum_algo: Crc<u32>) -> Self {
        Self {
            files,
            active_file,
            checksum_algo,
            index: 0,
            error: None,
        }
    }
    fn read_record(&mut self) -> Result<Vec<u8>, WALError> {
        if self.error.is_some() {
            return Err(self.error.clone().unwrap());
        }
        // as if file is not active next should return
        assert!(self.active_file.is_some());
        // as we know there is some active file
        let active_file = self.active_file.as_mut().unwrap();
        // will read the active file first and assume that the files are in order to which we need to read
        let lsn = match active_file.read_u64::<BigEndian>() {
            Ok(v) => v,
            Err(e) => {
                // as this this is the starting of the file the this can result to the EOF so we need to handle this
                match e.kind() {
                    std::io::ErrorKind::UnexpectedEof => {
                        // in this case we will
                        self.index += 1;
                        if self.index == self.files.len() {
                            // as there is not files
                            self.active_file = None;
                        } else {
                            let next_file =
                                File::options().read(true).open(&self.files[self.index])?;
                            self.active_file = Some(next_file);
                        }
                        // TODO: This recursion may cause issues
                        return self.read_record();
                    }
                    _ => {}
                }
                // other wise this is other error (non recoverable)
                return Err(e.into());
            }
        };
        let checksum = active_file.read_u32::<BigEndian>()?;
        let payload_len = active_file.read_u64::<BigEndian>()?;
        if payload_len > MAX_PAYLOAD_LEN {
            return Err(WALError::PayloadLengthOutOfBound(lsn));
        }

        let mut payload = vec![0; payload_len as usize];
        active_file.read_exact(&mut payload)?;
        // now we will read the magic number and check
        let magic_number = active_file.read_u64::<BigEndian>()?;
        let bytes_read = 8 + 4 + 8 + payload_len + 8;
        // after this we will do the 8 allignment
        active_file.seek(std::io::SeekFrom::Current(
            eight_align_addition(bytes_read) as i64
        ))?;

        if magic_number != MAGIC_NUMBER || checksum != self.checksum_algo.checksum(&payload) {
            let error = WALError::corruptedEntry(lsn);
            return Err(error);
        }
        return Ok(payload);
    }
}
impl Iterator for DefaultWALIterator {
    type Item = Result<Vec<u8>, WALError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.active_file.is_none() {
            return None;
        }
        if let Some(e) = &self.error {
            return Some(Err(e.clone()));
        }
        let payload = match self.read_record() {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(e.clone());
                return Some(Err(e));
            }
        };
        return Some(Ok(payload));
    }
}
impl WALIterator for DefaultWALIterator {}
