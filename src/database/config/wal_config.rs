use std::path::PathBuf;

use serde::Deserialize;
#[cfg(test)]
use tempfile::TempDir;

use crate::database::config::config_error::ConfigError;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WALVariant {
    Default,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WALConfig {
    pub wal_dir: PathBuf,
    pub variant: WALVariant,
    pub wal_group_sync_size: u64,
    pub wal_file_size_in_bytes: u64,
    pub wal_max_payload_len_in_bytes: u64,
}

impl WALConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.wal_group_sync_size == 0 {
            return Err(ConfigError::InvalidWALConfig(format!(
                "wal_group_sync_size must be > 0, got {}",
                self.wal_group_sync_size
            )));
        }

        if self.wal_max_payload_len_in_bytes == 0 {
            return Err(ConfigError::InvalidWALConfig(format!(
                "wal_max_payload_len_in_bytes must be > 0, got {}",
                self.wal_max_payload_len_in_bytes
            )));
        }

        if self.wal_file_size_in_bytes <= self.wal_max_payload_len_in_bytes * 10 {
            return Err(ConfigError::InvalidWALConfig(format!(
                "wal_file_size_in_bytes ({}) must be at least 10x wal_max_payload_len_in_bytes ({})",
                self.wal_file_size_in_bytes, self.wal_max_payload_len_in_bytes
            )));
        }
        Ok(())
    }
    #[cfg(test)]
    pub fn get_default_wal_test_config() -> (Self, TempDir) {
        let root_dir = TempDir::new().unwrap();
        (
            WALConfig {
                wal_dir: root_dir.path().join("wal").into(),
                variant: WALVariant::Default,
                wal_group_sync_size: 1,
                wal_file_size_in_bytes: 4 * 1024, // tiny for fast rotation
                wal_max_payload_len_in_bytes: 512,
            },
            root_dir,
        )
    }
}
