use std::{
    fs,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool, mpsc},
    thread::{JoinHandle, sleep},
    time::Duration,
};

use crate::database::{config::cleaner_config::CleanerConfig, sstable::version::Version};

pub struct Cleaner {
    config: CleanerConfig,
    under_shutdown: Arc<AtomicBool>,
    version_channel: mpsc::Receiver<(Arc<Version>, Vec<PathBuf>)>,
}

impl Cleaner {
    pub fn new(
        config: CleanerConfig,
        under_shutdown: Arc<AtomicBool>,
        version_channel: mpsc::Receiver<(Arc<Version>, Vec<PathBuf>)>,
    ) -> Self {
        Self {
            config,
            version_channel,
            under_shutdown,
        }
    }
    pub fn init(self) -> JoinHandle<u64> {
        std::thread::spawn(move || {
            let mut version_up_for_removal: Option<(Arc<Version>, Vec<PathBuf>)> = None;
            loop {
                if self
                    .under_shutdown
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    return 0;
                }
                if version_up_for_removal.is_none() {
                    // we will poll the mpsc
                    match self.version_channel.try_recv() {
                        Ok(v) => {
                            version_up_for_removal = Some(v);
                        }
                        Err(_e) => {}
                    }
                }
                if let Some((version, files)) = &version_up_for_removal {
                    // we will check if this is last copy of verion i.e nobody is using this version than we can remove the files
                    if Arc::strong_count(version) == 1 {
                        for file in files {
                            match fs::remove_file(file) {
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!("Error while deleting the file {:?}", e)
                                }
                            }
                        }
                        version_up_for_removal.take();
                    }
                }
                sleep(Duration::from_millis(self.config.cleaning_interval as u64));
            }
        })
    }
}
