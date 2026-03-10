use serde::{Deserialize, Serialize};

use crate::database::config::config_error::ConfigError;

/// Defines the in-memory data structure used for the memtable.
///
/// The memtable is the first storage layer in an LSM-tree.  
/// All writes are inserted into the memtable before eventually
/// being flushed to disk as SSTables.
///
/// Different structures provide different trade-offs:
/// - write speed
/// - memory overhead
/// - iteration efficiency
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MemtableVariant {
    /// Memtable implemented using a contiguous vector.
    ///
    /// Advantages:
    /// - very fast inserts
    /// - cache-friendly memory layout
    ///
    /// Disadvantages:
    /// - requires sorting during flush
    Vector,
    /// Memtable implemented using a B-Tree.
    ///
    /// Advantages:
    /// - maintains sorted keys
    /// - efficient range scans
    ///
    /// Disadvantages:
    /// - slightly slower inserts compared to vector/hash
    BTree,
    /// Memtable implemented using a hash map.
    ///
    /// Advantages:
    /// - extremely fast point inserts
    /// - efficient overwrites
    ///
    /// Disadvantages:
    /// - unordered structure
    /// - requires sorting before flush to SSTable
    Hash,
}

// TODO: confirm the memtable manager operation
/// Defines the memtable manager implementation.
///
/// The manager is responsible for:
/// - creating new memtables
/// - rotating memtables when full
/// - coordinating flush operations to disk
///
/// The enum allows multiple manager strategies in the future.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MemtableMangerVariant {
    /// Default memtable manager implementation.
    Default,
}

/// Configuration for the memtable subsystem.
///
/// The memtable temporarily stores writes in memory before
/// they are flushed to disk as SSTables.
#[derive(Debug, Deserialize, Serialize)]
pub struct MemtableConfig {
    /// The in-memory data structure used for the memtable.
    pub variant: MemtableVariant,
    /// Memtable manager implementation used by the engine.
    pub manager_variant: MemtableMangerVariant,
    /// Maximum size of a memtable in megabytes.
    ///
    /// Once the memtable exceeds this size, it is marked immutable
    ///
    /// Very small memtables can cause:
    /// - excessive flush operations
    /// - increased compaction pressure
    /// - poor write throughput
    pub max_memtable_size_in_mega_bytes: usize,
}

impl MemtableConfig {
    /// Validates the memtable configuration.
    ///
    /// Ensures the configuration values are reasonable before
    /// the database engine starts.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidMemtableConfig`] if the
    /// configuration contains invalid values.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.max_memtable_size_in_mega_bytes < 1 {
            return Err(ConfigError::InvalidMemtableConfig(format!(
                "Memtable size should be greater than 1 as smaller memtable will cause performance issue, provide {}",
                self.max_memtable_size_in_mega_bytes
            )));
        }
        Ok(())
    }
}
