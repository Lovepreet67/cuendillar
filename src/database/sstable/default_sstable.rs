use std::{
    fs::{File, create_dir_all},
    io::Read,
    path::PathBuf,
};

use crate::database::{OwnedEntry, memtable::Memtable, sstable::SSTable};

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
            .append(true)
            .open(metadata_path)?;
        Ok(Self {
            root_dir,
            metadata_file,
        })
    }
}
impl SSTable for DefaultSSTable {
    fn push_memtable(&mut self, mt: &impl Memtable) -> Result<(), std::io::Error> {
        let new_table_id = format!("{}", mt.get_id());
        let new_table_path = self.root_dir.join(&new_table_id);
        let mut writer = File::options()
            .append(true)
            .create_new(true)
            .open(new_table_path)?;
        for i in mt.iter() {
            i.encode(&mut writer)?;
        }
        Ok(())
    }
    fn build_memtable(&mut self, id: &uuid::Uuid) -> Result<Vec<OwnedEntry>, std::io::Error> {
        let table_path = self.root_dir.join(&id.to_string());
        let mut reader = File::options().read(true).open(table_path)?;
        let mut tor = Vec::new();
        while let Ok(entry) = OwnedEntry::decode(&mut reader) {
            tor.push(entry);
        }
        Ok(tor)
    }
}
