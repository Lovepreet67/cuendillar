use std::{path::PathBuf, sync::Arc};

use figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::Deserialize;
#[cfg(test)]
use tempfile::TempDir;

use crate::database::config::{
    bloom_config::BloomConfig, cleaner_config::CleanerConfig, compaction_config::CompactionConfig,
    config_error::ConfigError, index_config::IndexConfig, memtable_config::MemtableConfig,
    wal_config::WALConfig,
};

#[cfg(test)]
use crate::database::config::{
    bloom_config::BloomVariant,
    compaction_config::CompactionVariant,
    index_config::IndexVariant,
    memtable_config::{MemtableMangerVariant, MemtableVariant},
    wal_config::WALVariant,
};

pub mod bloom_config;
pub mod cleaner_config;
pub mod compaction_config;
pub mod config_error;
pub mod index_config;
pub mod memtable_config;
pub mod wal_config;

#[derive(Debug, Deserialize)]
pub struct DbConfig {
    pub root_dir: PathBuf,
    pub wal: WALConfig,
    pub memtable: MemtableConfig,
    pub bloom: BloomConfig,
    pub index: IndexConfig,
    pub compaction: CompactionConfig,
    pub cleaning: CleanerConfig,
}

impl DbConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.bloom.validate()?;
        self.cleaning.validate()?;
        self.compaction.validate()?;
        self.index.validate()?;
        self.memtable.validate()?;
        self.wal.validate()?;
        Ok(())
    }
    pub fn get_config() -> Result<Arc<DbConfig>, ConfigError> {
        let config_file_path =
            std::env::var("CONFIG_PATH").unwrap_or_else(|_| "./default_config.toml".to_owned());
        println!("Reading config from {:?}", config_file_path);
        let config: DbConfig = Figment::new()
            .merge(Toml::file(config_file_path))
            .extract()
            .expect("Failed to load DB config from path");
        config.validate()?;
        Ok(Arc::new(config))
    }

    #[cfg(test)]
    pub fn get_test_config() -> (Arc<DbConfig>, TempDir) {
        let root_dir = TempDir::new().unwrap();
        let cfg = DbConfig {
            root_dir: root_dir.path().to_path_buf(),
            wal: WALConfig {
                wal_dir: root_dir.path().join("wal").into(),
                variant: WALVariant::Default,
                wal_group_sync_size: 1,
                wal_file_size_in_bytes: 4 * 1024, // tiny for fast rotation
                wal_max_payload_len_in_bytes: 512,
            },
            memtable: MemtableConfig {
                variant: MemtableVariant::Vector,
                manager_variant: MemtableMangerVariant::Default,
                max_memtable_size: 10,
            },
            bloom: BloomConfig {
                variant: BloomVariant::Default,
                bits_per_key: 8,
                size: 128,
            },
            index: IndexConfig {
                variant: IndexVariant::Default,
                index_block_min_size: 1000,
            },
            compaction: CompactionConfig {
                root_dir: root_dir.path().into(),
                compaction_interval: 100,
                min_l0_file_count: 3,
                variant: CompactionVariant::Leveled,
                base_entries_per_table: 100,
                level_entries_growth_factor: 4,
                level_size_growth_factor: 4,
                level_base_size: 10000,
                max_level_count: 5,
            },
            cleaning: CleanerConfig {
                cleaning_interval: 1,
            },
        };
        (Arc::new(cfg), root_dir)
    }
}
