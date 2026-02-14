use serde::Deserialize;

use crate::database::config::config_error::ConfigError;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemtableVariant {
    Vector,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemtableMangerVariant {
    Default,
}

#[derive(Debug, Deserialize)]
pub struct MemtableConfig {
    pub variant: MemtableVariant,
    pub manager_variant: MemtableMangerVariant,
    pub max_memtable_size: usize,
}

impl MemtableConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.max_memtable_size <= 10 {
            return Err(ConfigError::InvalidMemtableConfig(format!(
                "Memtable size should be greater than 10 as smaller memtable will cause performance issue, provide {}",
                self.max_memtable_size
            )));
        }
        Ok(())
    }
}
