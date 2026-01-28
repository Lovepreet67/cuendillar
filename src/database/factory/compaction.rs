use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crate::database::{
    config::CompactionConfig,
    sstable::{
        compaction::{Compaction, leveled_compaction::LevelCompaction},
        version_manager::VersionManager,
    },
};

pub fn build_compaction(
    config: &CompactionConfig,
    root_dir: PathBuf,
    vm: Arc<RwLock<VersionManager>>,
    index_block_min_bytes: u64,
) -> Box<dyn Compaction> {
    match config.variant {
        crate::database::config::variants::CompactionVariant::Leveled => {
            Box::new(LevelCompaction::new(
                vm,
                config.min_l0_file_count,
                config.max_level_count,
                config.base_entries_per_table,
                config.level_entries_growth_factor,
                config.level_base_size,
                config.level_size_growth_factor,
                index_block_min_bytes,
                root_dir,
            ))
        }
    }
}
