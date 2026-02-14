use std::path::PathBuf;

use serde::Deserialize;

use crate::database::config::config_error::ConfigError;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactionVariant {
    Leveled,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompactionConfig {
    pub variant: CompactionVariant,
    pub root_dir: PathBuf,
    pub compaction_interval: usize,
    pub min_l0_file_count: usize,
    pub base_entries_per_table: usize,
    pub level_entries_growth_factor: usize,
    pub level_base_size: usize,
    pub level_size_growth_factor: usize,
    pub max_level_count: usize,
}

impl CompactionConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.compaction_interval == 0 {
            return Err(ConfigError::InvalidCompactionConfig(format!(
                "compaction_interval must be > 0, got {}",
                self.compaction_interval
            )));
        }

        if self.min_l0_file_count == 0 {
            return Err(ConfigError::InvalidCompactionConfig(format!(
                "min_l0_file_count must be > 0, got {}",
                self.min_l0_file_count
            )));
        }

        if self.base_entries_per_table <= 1 {
            return Err(ConfigError::InvalidCompactionConfig(format!(
                "base_entries_per_table must be > 1 (to support exponential growth across levels), got {}",
                self.base_entries_per_table
            )));
        }

        if self.level_entries_growth_factor <= 1 {
            return Err(ConfigError::InvalidCompactionConfig(format!(
                "level_entries_growth_factor must be > 1 (to support exponential growth across levels), got {}",
                self.level_entries_growth_factor
            )));
        }

        if self.level_base_size <= 1 {
            return Err(ConfigError::InvalidCompactionConfig(format!(
                "level_base_size must be > 1 (to support exponential growth across levels), got {}",
                self.level_base_size
            )));
        }

        if self.max_level_count == 0 || self.max_level_count > 10 {
            return Err(ConfigError::InvalidCompactionConfig(format!(
                "max_level_count must be between 1 and 10, got {}",
                self.max_level_count
            )));
        }

        Ok(())
    }
}
