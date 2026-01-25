use std::{
    fs,
    sync::{Arc, RwLock},
    thread::sleep,
    time::Duration,
};

use crate::database::sstable::version_manager::VersionManager;

pub struct Cleaner {
    version_manager: Arc<RwLock<VersionManager>>,
}

impl Cleaner {
    pub fn new(version_manager: Arc<RwLock<VersionManager>>) -> Self {
        Self { version_manager }
    }
    pub fn init(self) {
        std::thread::spawn(move || {
            loop {
                sleep(Duration::from_secs(3));
                let mut version_manger = self.version_manager.write().unwrap();
                let file_to_be_deleted = version_manger.claim();
                drop(version_manger);
                // now we will delete files onw by one
                for file in file_to_be_deleted {
                    fs::remove_file(file)
                        .map_err(|e| eprintln!("Error while deleting the file {:?}", e));
                }
            }
        });
    }
}
