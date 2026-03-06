use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use figment::{
    Figment,
    providers::{Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use tempfile::TempDir;
use tracing::info;

use crate::database::config::{
    bloom_config::BloomConfig, cleaner_config::CleanerConfig, compaction_config::CompactionConfig,
    config_error::ConfigError, index_config::IndexConfig, memtable_config::MemtableConfig,
    version_manager_config::VersionManagerConfig, wal_config::WALConfig,
};

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
pub mod version_manager_config;
pub mod wal_config;

#[derive(Debug, Deserialize, Serialize)]
pub struct DbConfig {
    pub root_dir: PathBuf,
    pub sstable_root_dir: PathBuf,
    pub wal: WALConfig,
    pub memtable: MemtableConfig,
    pub bloom: BloomConfig,
    pub index: IndexConfig,
    pub compaction: CompactionConfig,
    pub cleaning: CleanerConfig,
    pub version_manager: VersionManagerConfig,
}

impl DbConfig {
    pub fn get_dynamic_defaults(root_dir: &Path, sstable_root_dir: &Path) -> Self {
        Self {
            root_dir: root_dir.into(),
            sstable_root_dir: sstable_root_dir.into(),
            wal: WALConfig {
                wal_dir: root_dir.join("wal"),
                variant: WALVariant::Default,
                wal_file_size_in_bytes: 4 * 1024, // tiny for fast rotation
                wal_max_payload_len_in_bytes: 512,
                wal_sync_variant: wal_config::WALSyncVariant::NoSync,
            },
            memtable: MemtableConfig {
                variant: MemtableVariant::Vector,
                manager_variant: MemtableMangerVariant::Default,
                max_memtable_size_in_mega_bytes: 64,
            },
            bloom: BloomConfig {
                variant: BloomVariant::Default,
                bits_per_key: 8,
            },
            index: IndexConfig {
                variant: IndexVariant::Default,
                index_block_min_size: 1000,
            },
            compaction: CompactionConfig {
                root_dir: sstable_root_dir.into(),
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
                root_dir: sstable_root_dir.into(),
                cleaning_interval: 1,
            },
            version_manager: VersionManagerConfig {
                version_manager_sync_mode: version_manager_config::VersionMangerSyncVariant::NoSync,
            },
        }
    }
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.bloom.validate()?;
        self.cleaning.validate()?;
        self.compaction.validate()?;
        self.index.validate()?;
        self.memtable.validate()?;
        self.wal.validate()?;
        self.version_manager.validate()?;
        // all compacter, version_manager, cleaner should be on the same dir
        if self.compaction.root_dir != self.cleaning.root_dir {
            return Err(ConfigError::ExtractionError(
                "Compaction and cleaning are running on the different directories it should run on same".into(),
            ));
        }
        Ok(())
    }
    pub fn get_config() -> Result<Arc<DbConfig>, ConfigError> {
        let config_file_path =
            std::env::var("CONFIG_PATH").unwrap_or_else(|_| "./default_config.toml".to_owned());
        info!("Reading config from {:?}", config_file_path);
        // first we will generate the root_dir to get the default configs for some parts
        let partial_figment = Figment::new().merge(Toml::file(&config_file_path));
        // now we will get the root_dir and generate the default configs from it
        let root_dir: PathBuf = partial_figment.extract_inner("root_dir").map_err(|_e| {
            ConfigError::ExtractionError(format!("Root dir is required in the config"))
        })?;
        let sstable_root_dir: PathBuf = partial_figment
            .extract_inner("sstable_root_dir")
            .unwrap_or_else(|_| root_dir.join("sstable"));
        let dynamic_defaults = DbConfig::get_dynamic_defaults(&root_dir, &sstable_root_dir);
        // getting default configs
        let config: DbConfig = Figment::new()
            .merge(Serialized::defaults(dynamic_defaults))
            .merge(Toml::file(config_file_path))
            .extract()
            .map_err(|e| ConfigError::ExtractionError(format!("{:?}", e)))?;
        // TODO: all the paths should be canonicalized
        config.validate()?;
        Ok(Arc::new(config))
    }

    #[cfg(test)]
    pub fn get_test_config() -> (Arc<DbConfig>, TempDir) {
        let root_dir = TempDir::new().unwrap();
        let sstable_root_dir = root_dir.path().join("sstable");
        let cfg = Self::get_dynamic_defaults(root_dir.path(), &sstable_root_dir);
        (Arc::new(cfg), root_dir)
    }
}
