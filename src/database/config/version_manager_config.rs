use serde::{Deserialize, Serialize};
use tracing::error;

use crate::database::config::config_error::ConfigError;

// Currently WAL sync variant and this looks same but we may want to apply different constraints for them
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionMangerSyncVariant {
    NoSync,
    GroupSync(u64),
    Always,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VersionManagerConfig {
    pub version_manager_sync_mode: VersionMangerSyncVariant,
}

impl VersionManagerConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // There is nothing to check
        if let VersionMangerSyncVariant::GroupSync(x) = self.version_manager_sync_mode
            && x == 0
        {
            error!("Group size is set to 0 please use NoSync variant for better understandanbility")
        }
        Ok(())
    }
}
