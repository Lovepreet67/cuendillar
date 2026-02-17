use serde::{Deserialize, Serialize};

use crate::database::config::config_error::ConfigError;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexVariant {
    Default,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexConfig {
    pub variant: IndexVariant,
    pub index_block_min_size: usize,
}

impl IndexConfig {
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
