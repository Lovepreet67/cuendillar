use std::{
    fs::{File, create_dir_all},
    io::{BufReader, Write},
    path::PathBuf,
};

pub(crate) struct WAL {
    wal_dir: PathBuf,
    metadata_file: File,
    active_log: Option<File>,
}

impl WAL {
    pub fn new(wal_dir: PathBuf) -> Result<Self, std::io::Error> {
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
    pub fn read_log(&self, log_file_id: &str) -> Result<BufReader<File>, std::io::Error> {
        let log_file_path = self.wal_dir.join(log_file_id);
        let f = File::options()
            .read(true)
            .create(false)
            .open(log_file_path)?;
        Ok(BufReader::new(f))
    }
    pub fn append_log(&mut self, entry: &str) -> Result<Option<String>, std::io::Error> {
        let mut log_id = None;
        if self.active_log.is_none() {
            log_id = Some(self.new_log_file()?);
        }
        let active_log = self.active_log.as_mut().unwrap();
        active_log.write_all(entry.as_bytes())?;
        active_log.sync_data()?;
        Ok(log_id)
    }
    pub fn new_log_file(&mut self) -> Result<String, std::io::Error> {
        let new_log_file_id = format!("{}.wal", uuid::Uuid::new_v4());
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
        Ok(new_log_file_id)
    }
}

#[cfg(test)]
mod test {
    use std::{fs::remove_dir_all, io::BufRead, path::PathBuf, str::FromStr};

    use crate::database::wal::WAL;

    #[test]
    pub fn test_wal() {
        // create a new wal
        let wal_res = WAL::new(PathBuf::from_str("./wal").unwrap());
        assert!(wal_res.is_ok());
        let mut wal = wal_res.unwrap();
        let log_id_opt = wal.append_log("Testing a log\n").unwrap();
        assert!(log_id_opt.is_some());
        let log_id = log_id_opt.unwrap();
        wal.append_log("Testing a log1\n").unwrap();
        wal.append_log("Testing a log2\n").unwrap();
        wal.append_log("Testing a log3\n").unwrap();
        wal.append_log("Testing a log4\n").unwrap();
        let log_id2_res = wal.new_log_file();
        assert!(log_id2_res.is_ok());
        let mut first_log = wal.read_log(&log_id).unwrap();
        let mut buff = String::new();
        first_log.read_line(&mut buff).unwrap();
        assert_eq!(&buff, "Testing a log\n");
        buff.clear();
        first_log.read_line(&mut buff).unwrap();
        assert_eq!(&buff, "Testing a log1\n");
        buff.clear();
        first_log.read_line(&mut buff).unwrap();
        assert_eq!(&buff, "Testing a log2\n");
        buff.clear();
        first_log.read_line(&mut buff).unwrap();
        assert_eq!(&buff, "Testing a log3\n");
        buff.clear();
        first_log.read_line(&mut buff).unwrap();
        assert_eq!(&buff, "Testing a log4\n");
        drop(wal);
        // cleanup
        remove_dir_all("./wal").unwrap();
    }
}
