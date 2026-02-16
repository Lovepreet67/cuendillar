use std::{
    fs,
    sync::{Arc, RwLock, atomic::AtomicBool},
    thread::{JoinHandle, sleep},
    time::Duration,
};

use crate::database::{
    config::cleaner_config::CleanerConfig, sstable::version_manager::VersionManager,
};

pub struct Cleaner {
    config: CleanerConfig,
    version_manager: Arc<RwLock<VersionManager>>,
    under_shutdown: Arc<AtomicBool>,
}

impl Cleaner {
    pub fn new(
        config: CleanerConfig,
        version_manager: Arc<RwLock<VersionManager>>,
        under_shutdown: Arc<AtomicBool>,
    ) -> Self {
        Self {
            config,
            version_manager,
            under_shutdown,
        }
    }
    pub fn init(self) -> JoinHandle<u64> {
        std::thread::spawn(move || {
            loop {
                if self
                    .under_shutdown
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    return 0;
                }
                sleep(Duration::from_millis(self.config.cleaning_interval as u64));
                let mut version_manger = self.version_manager.write().unwrap();
                let file_to_be_deleted = version_manger.claim();
                drop(version_manger);
                // now we will delete files onw by one
                for file in file_to_be_deleted {
                    match fs::remove_file(file) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error while deleting the file {:?}", e)
                        }
                    }
                }
            }
        })
    }
}
