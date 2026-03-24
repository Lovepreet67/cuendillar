use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::database::config::config_error::ConfigError;

/// Defines the compaction strategy used by the database.
///
/// Compaction reorganizes on-disk tables (SSTables) to:
/// - remove obsolete entries
/// - apply tombstones
/// - merge overlapping data
/// - maintain read efficiency
///
/// Currently the engine supports **leveled compaction**, where data
/// is progressively merged from upper levels to lower levels.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactionVariant {
    /// Leveled compaction strategy.
    ///
    /// Data moves from **L0 → L1 → L2 → ... → Ln**.
    /// Each level becomes progressively larger while maintaining
    /// non-overlapping key ranges for efficient reads.
    Leveled,
}

/// Configuration for the compaction subsystem.
///
/// Compaction is responsible for maintaining the structure of the
/// LSM-tree by merging SSTables and removing obsolete data.
///
/// These parameters control:
/// - when compaction runs
/// - how large levels become
/// - how data grows across levels
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompactionConfig {
    /// Compaction strategy used by the database.
    pub variant: CompactionVariant,
    /// Root directory where SSTables are stored.
    ///
    /// The compaction process reads and writes SSTables
    /// inside this directory.
    pub root_dir: PathBuf,
    /// Interval between compaction runs.
    ///
    /// The background compaction worker periodically checks
    /// whether compaction should be triggered.
    ///
    /// The unit for this interval is ms
    pub compaction_interval: usize,
    /// Minimum number of files in **Level 0 (L0)** before compaction is triggered.
    ///
    /// L0 files may overlap in key ranges, which makes reads slower.
    /// Once the number of L0 files exceeds this threshold,
    /// a compaction into L1 will be scheduled.
    pub min_l0_file_count: usize,
    /// Maximum number of files in **Level 0 (L0)** which will be taken out from L0 during compaction start
    pub max_l0_file_count_per_cycle: usize,
    /// Base number of entries in table it will be equal to the number of entries
    /// in table in L0 level
    /// this number will grow based on growth factor
    pub base_entries_per_table: usize,
    /// Growth factor for the number of entries allowed per level.
    ///
    /// Each level can hold more entries than the previous one.
    /// For example with a factor of 10:
    ///
    /// - L1 = 10 × L0 capacity
    /// - L2 = 10 × L1 capacity
    /// - L3 = 10 × L2 capacity
    ///
    /// This exponential growth keeps the number of levels small
    /// while allowing the database to scale.
    pub level_entries_growth_factor: usize,
    /// Base size of Level 1.
    ///
    /// This value defines the starting size for the first leveled
    /// compaction tier.
    pub level_base_size: usize,
    /// Growth factor controlling how level sizes increase.
    ///
    /// Each level is larger than the previous one by this factor.
    ///
    /// Example with growth factor = 10:
    ///
    /// - L1 = base size
    /// - L2 = 10 × L1
    /// - L3 = 10 × L2
    pub level_size_growth_factor: usize,
    /// Maximum number of levels allowed in the LSM tree.
    ///
    /// Typical LSM trees use between **5 and 7 levels**.
    /// Increasing this value allows more data to be stored
    /// but may increase read amplification.
    pub max_level_count: usize,
}

impl CompactionConfig {
    /// Validates the compaction configuration.
    ///
    /// Ensures that parameters are within acceptable bounds
    /// before the database engine starts.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidCompactionConfig`] if any
    /// configuration value is invalid.
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
