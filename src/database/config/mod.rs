use std::path::PathBuf;

use figment::{
    Figment,
    providers::{Format, Toml},
};
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::database::config::variants::{
    BloomVariant, CompactionVariant, IndexVariant, MemtableVariant, SSTableVariant, WALVariant,
};

pub mod variants;

#[derive(Debug, Deserialize)]
pub struct WALConfig {
    pub variant: WALVariant,
}

#[derive(Debug, Deserialize)]
pub struct MemtableConfig {
    pub variant: MemtableVariant,
    pub max_memtable_size: u64,
}
#[derive(Debug, Deserialize)]
pub struct SSTableConfig {
    pub variant: SSTableVariant,
}
#[derive(Debug, Deserialize)]
pub struct IndexConfig {
    pub variant: IndexVariant,
}
#[derive(Debug, Deserialize)]
pub struct BloomConfig {
    pub variant: BloomVariant,
    pub key_size: u64,
}
#[derive(Debug, Deserialize)]
pub struct CompactionConfig {
    pub variant: CompactionVariant,
    pub min_l0_file_count: u64,
    pub base_entries_per_table: u64,
    pub level_size_growth_factor: u64,
    pub max_level_count: u64,
}
#[derive(Debug, Deserialize)]
pub struct DbConfig {
    pub root_dir: PathBuf,
    pub wal: WALConfig,
    pub memtable: MemtableConfig,
    pub sstable: SSTableConfig,
    pub bloom: BloomConfig,
    pub index: IndexConfig,
    pub compaction: CompactionConfig,
}

pub static CONFIG: Lazy<DbConfig> = Lazy::new(|| {
    let config_file_path =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "./default_config.toml".to_owned());
    eprintln!("{:?}", config_file_path);
    Figment::new()
        .merge(Toml::file(config_file_path))
        .extract()
        .expect("Failed to load DB config from path")
});

#[cfg(test)]
mod test {
    use crate::database::config::CONFIG;

    #[test]
    fn test_default_config() {
        eprintln!("{:?}", CONFIG.bloom);
    }
}
