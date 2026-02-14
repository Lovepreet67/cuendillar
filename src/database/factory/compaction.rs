use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crate::database::{
    config::{
        bloom_config::BloomConfig, compaction_config::CompactionConfig, index_config::IndexConfig,
    },
    sstable::{
        compaction::{Compaction, leveled_compaction::LevelCompaction},
        version_manager::VersionManager,
    },
};

pub fn build_compaction(
    config: &CompactionConfig,
    bloom_config: &BloomConfig,
    index_config: &IndexConfig,
    vm: Arc<RwLock<VersionManager>>,
) -> Box<dyn Compaction> {
    match config.variant {
        crate::database::config::compaction_config::CompactionVariant::Leveled => {
            Box::new(LevelCompaction::new(vm, config, bloom_config, index_config))
        }
    }
}
