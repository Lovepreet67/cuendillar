use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::database::config::config_error::ConfigError;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CleanerConfig {
    pub root_dir: PathBuf,
    pub cleaning_interval: usize,
}

impl CleanerConfig {
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
