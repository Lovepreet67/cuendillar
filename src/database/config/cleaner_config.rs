use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::database::config::config_error::ConfigError;

/// Configuration for the background cleaner.
///
/// The cleaner is responsible for periodically removing obsolete
/// SSTable files created by the database engine.
///
/// Cleaner get the files and Arc<Version> from Version Manager through an mpsc channel,
/// Once the cleaner confirm that it is sole owner of the Version it drop that version and delete the files.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CleanerConfig {
    /// Root directory of the database.
    ///
    /// The cleaner scans this directory for files that can be safely
    /// removed based on the database's internal metadata.
    pub root_dir: PathBuf,
    /// Interval at which the cleaner runs.
    ///
    /// This value determines how frequently the cleaner wakes up and try_recv from mpsc channel
    /// Once cleaner get value of recv it will clear it and then try to read again after 1/10th of specified interval.
    /// In case of read miss the cleaner will sleep for specified interval
    /// The unit of this value is ms
    pub cleaning_interval: usize,
}

impl CleanerConfig {
    /// Validates the cleaner configuration.
    ///
    /// Ensures that the provided configuration values are valid
    /// before the database engine starts.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidCleanerConfig`] if the
    /// `cleaning_interval` is zero.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.cleaning_interval <= 0 {
            return Err(ConfigError::InvalidCleanerConfig(format!(
                "Cleaning Interval should be greater than 0, provide {}",
                self.cleaning_interval
            )));
        }
        Ok(())
    }
}
