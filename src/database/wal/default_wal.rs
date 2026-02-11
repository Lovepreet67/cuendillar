use std::{
    fs::{File, create_dir_all, read_dir, remove_file},
    io::{BufReader, Read, Seek, Write},
    path::PathBuf,
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crc::Crc;

use crate::database::{
    config::CONFIG,
    wal::{MAGIC_NUMBER, WAL, WALIterator, errors::WALError},
};

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
    curr_offset: u64, // this will be used for lsn number, it will be globaly increasing accross files
    last_rotation_offset: u64, // offset at which last rotation happened
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
            last_rotation_offset: 0,
            crc_computer: Crc::<u32>::new(&crc::CRC_32_CKSUM),
        };
        let files = wal.get_all_files()?;
        let (current_offset, active_log_file) = if files.len() == 0 {
            let new_file_path = wal.wal_dir.join("0.wal");
            let new_file = File::options()
                .create(true)
                .append(true)
                .open(new_file_path)?;
            (0, new_file)
        } else {
            // here we will replay the logs
            let active_file = File::options().read(true).open(&files[0].1)?;
            // curr offset is the first offset from which next log will start
            let mut curr_offset = files[0].0;
            let wal_iterator = DefaultWALIterator::new(
                files.iter().skip(1).map(|item| item.1.clone()).collect(),
                Some(BufReader::new(active_file)),
                wal.crc_computer.clone(),
            );
            // now we will replay the wal
            for log in wal_iterator {
                match log {
                    Ok(v) => curr_offset = v.0 + Self::get_entry_size(v.1.len() as u64),
                    Err(e) => {
                        eprintln!(
                            "Trimming the wal at {} because of error {:?}",
                            curr_offset, e
                        );
                        break;
                    }
                }
            }

            // now we have current offset we will find the file which contain this offset
            let file_index = match files.binary_search_by(|item| {
                if item.0 < curr_offset {
                    std::cmp::Ordering::Less
                } else if item.0 == curr_offset {
                    std::cmp::Ordering::Equal
                } else {
                    std::cmp::Ordering::Greater
                }
            }) {
                Ok(target_file_index) => target_file_index,
                Err(next_file_index) => {
                    // next_file_index will always be greater than 0 as we started iterator from first file
                    next_file_index - 1
                }
            };

            if file_index != files.len() - 1 {
                eprintln!(
                    "Can't start DB, corruption found other than last file in WAL file {:?}",
                    files[file_index].1
                );
                return Err(WALError::CorruptedEntry(curr_offset));
            }

            // wal.offset is the starting offset of the last valid log

            let active_file = File::options().write(true).open(&files[file_index].1)?;
            assert!(curr_offset >= files[file_index].0);
            let index_inside_file = curr_offset - files[file_index].0;
            active_file.set_len(index_inside_file)?;
            active_file.sync_all()?;
            drop(active_file);
            let active_file = File::options().append(true).open(&files[file_index].1)?;
            wal.last_rotation_offset = curr_offset; // Marking last rotation offset we will not rotate here because there will be read operation first
            (curr_offset, active_file)
        };
        wal.active_log = Some(active_log_file);
        wal.curr_offset = current_offset;
        Ok(wal)
    }
    pub fn get_entry_size(payload_len: u64) -> u64 {
        // lsn(u64)+crc(u32)+payload_len(u64)+payload_+magic_number(u64)+padding
        let data_len = 8 + 4 + 8 + payload_len + 8;
        data_len + eight_align_addition(data_len)
    }
    pub fn rotate(&mut self) -> Result<(), WALError> {
        let new_log_file_id = format!("{}.wal", self.curr_offset);
        let new_log_file_path = self.wal_dir.join(&new_log_file_id);
        let new_log_file = File::options()
            .create_new(true)
            .append(true)
            .open(new_log_file_path)?;
        if let Some(active_log) = self.active_log.take() {
            active_log.sync_all()?;
            drop(active_log);
        }
        new_log_file.sync_all()?;
        self.active_log = Some(new_log_file);
        self.last_rotation_offset = self.curr_offset;
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
    fn append_log(&mut self, payload: &[u8]) -> Result<u64, WALError> {
        if payload.len() as u64 > CONFIG.wal.wal_max_payload_len {
            return Err(WALError::PayloadLengthOutOfBound(payload.len() as u64));
        }
        // do rotation in case of file size exceed
        if self.active_log.is_none()
            || self.curr_offset - self.last_rotation_offset > CONFIG.wal.wal_file_size
        {
            self.rotate()?;
        }
        // we will create a buffer localy so that we can do write all
        let mut local_buff = Vec::with_capacity(8 + 4 + 8 + payload.len() + 8 + 8); // last 8 for padding 
        local_buff.write_u64::<BigEndian>(self.curr_offset)?;
        local_buff.write_u32::<BigEndian>(self.crc_computer.checksum(&payload))?;
        local_buff.write_u64::<BigEndian>(payload.len() as u64)?;
        local_buff.write_all(&payload)?;
        local_buff.write_u64::<BigEndian>(MAGIC_NUMBER)?;
        // we will make this localbuff allign to 8
        while local_buff.len() % 8 != 0 {
            local_buff.write_u8(0)?;
        }
        let active_log = self.active_log.as_mut().unwrap();
        active_log.write_all(&local_buff)?;
        self.curr_offset += local_buff.len() as u64;
        self.counter += 1;
        if self.counter >= self.wal_sync_group_size {
            self.counter = 0;
            active_log.sync_data()?;
        }
        Ok(self.curr_offset)
    }
    fn read(&self, offset: u64) -> Result<Box<dyn WALIterator>, WALError> {
        let files = self.get_all_files()?;
        if files.len() == 0 {
            return Ok(Box::new(DefaultWALIterator::new(
                vec![],
                None,
                self.crc_computer.clone(),
            )));
        }

        // file is the first file which have starting offset > the specified offset
        // so the files that will be contain the logs for specified offset is file_index-1 .. to end
        let mut file_index = 0;
        while file_index < files.len() && offset >= files[file_index].0 {
            file_index += 1;
        }
        if file_index == 0 {
            return Err(WALError::OffsetUnderflow);
        }
        let offset_inside_file = offset - files[file_index - 1].0;
        let files_to_be_included = files
            .into_iter()
            .skip(file_index - 1) // skip files which doesn't contain the offset
            .map(|item| item.1)
            .collect::<Vec<PathBuf>>();
        // now we will open the first file and move the pointer to the lsn to
        let mut active_file = File::options().read(true).open(&files_to_be_included[0])?;
        // we must check if the specified offset is in the file bounderies or not
        // TODO: check if the offset is legal for the specified file or not
        // Posix allow seeking beyond file EOF we have to check to detect file corruption
        active_file.seek(std::io::SeekFrom::Start(offset_inside_file))?;
        return Ok(Box::new(DefaultWALIterator::new(
            files_to_be_included,
            Some(BufReader::new(active_file)),
            self.crc_computer.clone(),
        )));
    }
    fn flush_wal(&mut self, offset: u64) -> Result<(), WALError> {
        // we will delete all the files which contain offset less than the offset provided and remove them
        let max_offset_to_be_flushed = offset - 1;
        let files = self.get_all_files()?;
        let mut file_index = 0;
        while file_index < files.len() && max_offset_to_be_flushed > files[file_index].0 {
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
    active_file: Option<BufReader<File>>,
    error: Option<WALError>,
    checksum_algo: Crc<u32>,
    index: usize,
}
impl DefaultWALIterator {
    pub fn new(
        files: Vec<PathBuf>,
        active_file: Option<BufReader<File>>,
        checksum_algo: Crc<u32>,
    ) -> Self {
        Self {
            files,
            active_file,
            checksum_algo,
            index: 0,
            error: None,
        }
    }
    fn read_record(&mut self) -> Result<Option<(u64, Vec<u8>)>, WALError> {
        if self.error.is_some() {
            return Err(self.error.clone().unwrap());
        }
        // as if file is not active next should return
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
                        if self.index >= self.files.len() {
                            // as there is not files
                            self.active_file = None;
                            return Ok(None);
                        } else {
                            let next_file =
                                File::options().read(true).open(&self.files[self.index])?;
                            self.active_file = Some(BufReader::new(next_file));
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
        if payload_len > CONFIG.wal.wal_max_payload_len {
            return Err(WALError::PayloadLengthOutOfBound(lsn));
        }

        let mut payload = vec![0; payload_len as usize];
        active_file.read_exact(&mut payload)?;
        // now we will read the magic number and check
        let magic_number = active_file.read_u64::<BigEndian>()?;
        let bytes_read = 8 + 4 + 8 + payload_len + 8;

        // after this we will do the 8 allignment
        let pad = eight_align_addition(bytes_read) as i64;
        if pad > 0 {
            let mut skip = [0u8; 8];
            active_file.read_exact(&mut skip[..pad as usize])?;
        }

        if magic_number != MAGIC_NUMBER || checksum != self.checksum_algo.checksum(&payload) {
            let error = WALError::CorruptedEntry(lsn);
            return Err(error);
        }
        return Ok(Some((lsn, payload)));
    }
}
impl Iterator for DefaultWALIterator {
    type Item = Result<(u64, Vec<u8>), WALError>;
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
        match payload {
            Some(v) => Some(Ok(v)),
            _ => None,
        }
    }
}
impl WALIterator for DefaultWALIterator {}
