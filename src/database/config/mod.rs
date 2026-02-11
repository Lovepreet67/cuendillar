use std::path::PathBuf;

use figment::{
    Figment,
    providers::{Format, Toml},
};
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::database::config::variants::{
    BloomVariant, CompactionVariant, IndexVariant, MemtableMangerVariant, MemtableVariant,
    WALVariant,
};

pub mod variants;

#[derive(Debug, Deserialize)]
pub struct WALConfig {
    pub variant: WALVariant,
    pub wal_group_sync_size: u64,
    pub wal_file_size: u64,
    pub wal_max_payload_len: u64,
}

#[derive(Debug, Deserialize)]
pub struct MemtableConfig {
    pub variant: MemtableVariant,
    pub manager_variant: MemtableMangerVariant,
    pub max_memtable_size: u64,
}
#[derive(Debug, Deserialize)]
pub struct IndexConfig {
    pub variant: IndexVariant,
}
#[derive(Debug, Deserialize)]
pub struct BloomConfig {
    pub variant: BloomVariant,
    pub key_size: u32,
    pub size: u32,
}
#[derive(Debug, Deserialize)]
pub struct CompactionConfig {
    pub variant: CompactionVariant,
    pub compaction_interval: u64,
    pub min_l0_file_count: u16,
    pub base_entries_per_table: u16,
    pub level_entries_growth_factor: u16,
    pub level_base_size: u64,
    pub level_size_growth_factor: u64,
    pub max_level_count: u16,
}
#[derive(Debug, Deserialize)]
pub struct DbConfig {
    pub root_dir: PathBuf,
    pub index_block_min_size: u64,
    pub wal: WALConfig,
    pub memtable: MemtableConfig,
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
