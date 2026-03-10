use std::path::PathBuf;

use serde::{Deserialize, Serialize};
#[cfg(test)]
use tempfile::TempDir;

use crate::database::config::{
    config_error::ConfigError, version_manager_config::VersionMangerSyncVariant,
};

/// Defines the WAL implementation used by the database.
///
/// The Write-Ahead Log (WAL) ensures durability by recording
/// write operations before they are applied to the memtable.
///
/// In case of a crash, the WAL is replayed to rebuild the
/// in-memory state of the database.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WALVariant {
    /// Default WAL implementation.
    Default,
}

/// Defines the synchronization policy used by the WAL.
///
/// Synchronization controls how frequently WAL writes are
/// flushed to disk using `fsync` or an equivalent operation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WALSyncVariant {
    /// Do not explicitly synchronize WAL writes.
    ///
    /// This provides the highest write throughput but risks
    /// losing recent writes if the system crashes.
    NoSync,
    /// Synchronize WAL writes in batches.
    ///
    /// The provided value specifies the number of operations
    /// that may accumulate before a disk sync occurs.
    ///
    /// Example:
    /// `GroupSync(100)` means the WAL will flush to disk
    /// after roughly 100 writes.
    GroupSync(u64),
    /// Always synchronize WAL writes immediately.
    ///
    /// This provides the strongest durability guarantees
    /// but may significantly reduce write throughput.
    Always,
}

/// Allows conversion from the Version Manager's sync policy
/// to the WAL sync policy.
///
/// Although both policies currently share similar variants,
/// they are defined separately so that independent constraints
/// can be introduced in the future.
impl From<VersionMangerSyncVariant> for WALSyncVariant {
    fn from(value: VersionMangerSyncVariant) -> Self {
        match value {
            VersionMangerSyncVariant::Always => WALSyncVariant::Always,
            VersionMangerSyncVariant::GroupSync(x) => WALSyncVariant::GroupSync(x),
            VersionMangerSyncVariant::NoSync => WALSyncVariant::NoSync,
        }
    }
}

/// Configuration for the Write-Ahead Log (WAL).
///
/// The WAL records all write operations before they are applied
/// to the memtable, ensuring durability and enabling crash recovery.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WALConfig {
    /// Directory where WAL files are stored.
    pub wal_dir: PathBuf,
    /// WAL implementation variant.
    pub variant: WALVariant,
    /// Maximum size of a single WAL file in bytes.
    ///
    /// Once this limit is reached, the WAL will rotate and
    /// a new log file will be created.
    pub wal_file_size_in_bytes: u64,
    /// Maximum payload size for a single WAL record in bytes.
    ///
    /// This prevents extremely large records from being written
    /// to the WAL, which helps maintain predictable performance
    /// and helps in detecting the corrupted data in wal.
    pub wal_max_payload_len_in_bytes: u64,
    /// Synchronization strategy used for WAL writes.
    pub wal_sync_variant: WALSyncVariant,
}

impl WALConfig {
    /// Validates the WAL configuration.
    ///
    /// Ensures configuration values are within acceptable bounds
    /// before the database engine starts.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidWALConfig`] if configuration
    /// parameters violate required constraints.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let WALSyncVariant::GroupSync(x) = self.wal_sync_variant
            && x == 0
        {
            eprintln!(
                "Group size is set to 0 please use NoSync variant for better understandanbility"
            )
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
    /// Returns a minimal WAL configuration for testing.
    ///
    /// This helper creates a temporary directory and a WAL
    /// configuration with very small file sizes to trigger
    /// frequent WAL rotation during tests.
    ///
    /// The temporary directory is returned alongside the config
    /// to ensure it remains alive for the duration of the test.
    #[cfg(test)]
    pub fn get_default_wal_test_config() -> (Self, TempDir) {
        let root_dir = TempDir::new().unwrap();
        (
            WALConfig {
                wal_dir: root_dir.path().into(),
                variant: WALVariant::Default,
                wal_file_size_in_bytes: 4 * 1024, // tiny for fast rotation
                wal_max_payload_len_in_bytes: 512,
                wal_sync_variant: WALSyncVariant::NoSync,
            },
            root_dir,
        )
    }
}
