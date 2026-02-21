use serde::{Deserialize, Serialize};

use crate::database::config::config_error::ConfigError;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MemtableVariant {
    Vector,
    BTree,
    Hash,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MemtableMangerVariant {
    Default,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MemtableConfig {
    pub variant: MemtableVariant,
    pub manager_variant: MemtableMangerVariant,
    pub max_memtable_size_in_mega_bytes: usize,
}

impl MemtableConfig {
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
