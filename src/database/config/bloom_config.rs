use serde::{Deserialize, Serialize};

use crate::database::config::config_error::ConfigError;

/// Defines the Bloom filter implementation used by the database.
///
/// Bloom filters are probabilistic data structures used to quickly
/// determine whether a key **may exist** in an SSTable.
///
/// They help avoid unnecessary disk reads by allowing the engine
/// to skip files that definitely do not contain the requested key.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BloomVariant {
    /// Default Bloom filter implementation used by the database.
    Default,
}

// Configuration for Bloom filters used by the storage engine.
///
/// Bloom filters are typically created per SSTable and used during
/// read operations to quickly determine whether the SSTable might
/// contain a given key.
///
/// Proper tuning of Bloom filters can significantly improve read
/// performance by reducing unnecessary disk I/O.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BloomConfig {
    /// Bloom filter implementation variant.
    pub variant: BloomVariant,
    /// Number of bits allocated per key in the Bloom filter.
    ///
    /// This parameter controls the **false positive rate** of the filter.
    ///
    /// Larger values:
    /// - reduce false positives
    /// - increase memory usage
    ///
    /// Smaller values:
    /// - reduce memory usage
    /// - increase false positives
    ///
    /// Typical values range from **8–12 bits per key**.
    pub bits_per_key: usize,
}

impl BloomConfig {
    /// Validates the Bloom filter configuration.
    ///
    /// Ensures that configuration parameters are within valid bounds.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidBloomConfig`] if the configuration
    /// is invalid.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.bits_per_key == 0 {
            return Err(ConfigError::InvalidBloomConfig(format!(
                "Key Size should be greater than 0, provide {}",
                self.bits_per_key
            )));
        }
        Ok(())
    }
    /// Returns a Bloom configuration suitable for tests.
    ///
    /// This configuration uses a small Bloom filter with
    /// `8 bits per key`, which provides a reasonable balance
    /// between memory usage and false positive rate for tests.
    pub fn get_test_config() -> Self {
        BloomConfig {
            variant: BloomVariant::Default,
            bits_per_key: 8,
        }
    }
}

impl From<&str> for BloomVariant {
    fn from(value: &str) -> Self {
        match value {
            "Default" => BloomVariant::Default,
            _ => BloomVariant::Default,
        }
    }
}
