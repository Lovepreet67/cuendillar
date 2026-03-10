use serde::{Deserialize, Serialize};
use tracing::error;

use crate::database::config::config_error::ConfigError;

/// Defines how the Version Manager persists metadata changes.
///
/// The Version Manager manages metadata regarding
/// - SSTable manifests
/// - level layouts
/// - compaction results
///
/// These updates may need to be synchronized to disk depending
/// on the durability guarantees required by the system.
///
/// This enum controls how frequently those updates are synced.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionMangerSyncVariant {
    /// Do not explicitly sync metadata updates to disk.
    ///
    /// This provides the highest performance but risks losing
    /// recent metadata updates in case of a crash.
    NoSync,
    /// Synchronize metadata updates in groups.
    ///
    /// The provided value represents the number of operations
    /// that may accumulate before a sync is triggered.
    ///
    /// Example:
    /// `GroupSync(100)` means the system will flush metadata
    /// changes to disk after approximately 100 updates.
    GroupSync(u64),
    /// Always sync metadata updates to disk immediately.
    ///
    /// This provides the strongest durability guarantees
    /// but may significantly reduce write throughput.
    Always,
}

/// Configuration for the Version Manager.
///
/// The Version Manager is responsible for maintaining the
/// metadata state of the LSM tree, including the mapping
/// between levels and SSTables.
#[derive(Debug, Deserialize, Serialize)]
pub struct VersionManagerConfig {
    /// Synchronization strategy used for version metadata.
    pub version_manager_sync_mode: VersionMangerSyncVariant,
}

impl VersionManagerConfig {
    /// Validates the version manager configuration.
    ///
    /// Currently only performs a sanity check for the `GroupSync`
    /// variant to ensure the group size is meaningful.
    ///
    /// # Errors
    ///
    /// This function currently does not return an error but may
    /// emit warnings through logging if suspicious values are used.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // There is nothing to check
        if let VersionMangerSyncVariant::GroupSync(x) = self.version_manager_sync_mode
            && x == 0
        {
            error!("Group size is set to 0 please use NoSync variant for better understandanbility")
        }
        Ok(())
    }
}
