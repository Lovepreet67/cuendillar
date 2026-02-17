use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub enum ConfigError {
    InvalidBloomConfig(String),
    InvalidCleanerConfig(String),
    InvalidCompactionConfig(String),
    InvalidIndexConfig(String),
    InvalidMemtableConfig(String),
    InvalidWALConfig(String),
    ExtractionError(String),
}
