use crate::database::{
    config::wal_config::{WALConfig, WALVariant},
    wal::{WAL, default_wal::DefaultWAL, errors::WALError},
};

pub fn build_wal_manger(wal_config: &WALConfig) -> Result<Box<dyn WAL>, WALError> {
    match wal_config.variant {
        WALVariant::Default => {
            let wal = DefaultWAL::new(wal_config)?;
            return Ok(Box::new(wal));
        }
    }
}
