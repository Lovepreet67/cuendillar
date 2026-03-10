use serde::{Deserialize, Serialize};

use crate::database::config::config_error::ConfigError;

/// Defines the index implementation used by the database.
///
/// Index structures are used inside SSTables to efficiently locate
/// data blocks containing specific keys without scanning the entire file.
///
/// Currently the engine provides a default index implementation,
/// but the enum allows additional index formats to be added in the future.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexVariant {
    /// Default index implementation used by the database.
    Default,
}

/// Configuration for the SSTable index structure.
///
/// Each SSTable contains an index that maps key ranges to data blocks.
/// During reads, the engine consults the index to determine which block
/// might contain the requested key.
///
/// Proper tuning of index parameters can improve read performance
/// while controlling memory and disk overhead.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexConfig {
    /// Index implementation used by the database.
    pub variant: IndexVariant,
    /// Minimum size (in bytes) for index blocks.
    ///
    /// Index entries are grouped into blocks to reduce overhead.
    /// Once the accumulated size of entries reaches this threshold,
    /// a new index block will be created.
    ///
    /// Larger blocks:
    /// - reduce index metadata overhead
    /// - may increase lookup time slightly
    ///
    /// Smaller blocks:
    /// - improve lookup precision
    /// - increase metadata size
    pub index_block_min_size: usize,
}

impl IndexConfig {
    /// Validates the index configuration.
    ///
    /// Ensures configuration parameters are within valid bounds
    /// before the database engine starts.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidIndexConfig`] if the
    /// configuration contains invalid values.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.index_block_min_size == 0 {
            return Err(ConfigError::InvalidIndexConfig(format!(
                "index_block_min_size should be greater than 0, provide {}",
                self.index_block_min_size
            )));
        }
        Ok(())
    }
}

impl From<&str> for IndexVariant {
    fn from(value: &str) -> Self {
        match value {
            "Default" => IndexVariant::Default,
            _ => IndexVariant::Default,
        }
    }
}
