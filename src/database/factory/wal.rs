use std::path::PathBuf;

use crate::database::{
    config::{self, WALConfig, variants::WALVariant},
    wal::{WAL, default_wal::DefaultWAL, errors::WALError},
};

pub fn build_wal_manger(
    wal_config: &WALConfig,
    wal_dir: PathBuf,
) -> Result<Box<dyn WAL>, WALError> {
    match wal_config.variant {
        WALVariant::Default => {
            let wal = DefaultWAL::new(wal_dir, wal_config.wal_group_sync_size)?;
            return Ok(Box::new(wal));
        }
    }
}
