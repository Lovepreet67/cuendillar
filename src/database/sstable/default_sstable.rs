use std::{
    fs::{File, create_dir_all},
    io::{BufRead, BufReader, Seek, Write},
    path::PathBuf,
};

use crate::database::{
    OwnedEntry,
    memtable::Memtable,
    sstable::{SSTable, errors::SSTableError},
};

pub struct DefaultSSTable {
    root_dir: PathBuf,
    metadata_file: File,
}

impl DefaultSSTable {
    pub fn new(root_dir: PathBuf) -> Result<Self, std::io::Error> {
        // first we will check if the parrent dir exists or not
        if !root_dir.exists() {
            create_dir_all(&root_dir)?;
        }
        let metadata_path = root_dir.join("metadata.sst");
        let metadata_file = File::options()
            .create(true)
            .read(true)
            .write(true)
            .open(metadata_path)?;
        Ok(Self {
            root_dir,
            metadata_file,
        })
    }
}
impl SSTable for DefaultSSTable {
    fn push_memtable(&mut self, mt: &impl Memtable) -> Result<(), SSTableError> {
        let new_table_id = format!("{}", mt.get_id());
        let new_table_path = self.root_dir.join(&new_table_id);
        let mut writer = File::options()
            .append(true)
            .create_new(true)
            .open(new_table_path)?;
        self.metadata_file.write_all(new_table_id.as_bytes())?;
        self.metadata_file.write_all(b"\n")?;
        for i in mt.iter() {
            i.encode(&mut writer)?;
        }
        Ok(())
    }
    fn find(&mut self, id: &[u8]) -> Result<Option<OwnedEntry>, SSTableError> {
        // currently we are assuming all the files are in L0 so we will do brute force search
        // we will read the metadata file and then will go bottom to top scanning each memtable
        let curr_pos = self.metadata_file.seek(std::io::SeekFrom::Current(0))?;
        self.metadata_file.seek(std::io::SeekFrom::Start(0))?;
        let buf = BufReader::new(&mut self.metadata_file);
        let mut available_sstable = vec![];
        for i in buf.lines() {
            if i.is_err() {
                println!("{:?}", i);
                break;
            }
            let line = i?;
            available_sstable.push(line);
        }
        self.metadata_file
            .seek(std::io::SeekFrom::Start(curr_pos))?;
        available_sstable = available_sstable.into_iter().rev().collect();
        for table_id in available_sstable {
            // we will build memtable and search in that
            let table_path = self.root_dir.join(&table_id.to_string());
            let mut reader = File::options().read(true).open(table_path)?;
            while let Ok(entry) = OwnedEntry::decode(&mut reader) {
                if id == entry.get_id() {
                    return Ok(Some(entry));
                }
            }
        }
        return Ok(None);
    }
}
