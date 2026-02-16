use serde::{Deserialize, Serialize};

use crate::database::config::config_error::ConfigError;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BloomVariant {
    Default,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BloomConfig {
    pub variant: BloomVariant,
    pub size: usize,
    pub bits_per_key: usize,
}

impl BloomConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.bits_per_key == 0 {
            return Err(ConfigError::InvalidBloomConfig(format!(
                "Key Size should be greater than 0, provide {}",
                self.bits_per_key
            )));
        }
        if self.size == 0 {
            return Err(ConfigError::InvalidBloomConfig(format!(
                "Bloom Size should be greater than 0, provide {}",
                self.size
            )));
        }
        Ok(())
    }
    pub fn get_test_config() -> Self {
        BloomConfig {
            variant: BloomVariant::Default,
            bits_per_key: 8,
            size: 128,
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
