use std::path::PathBuf;

use thiserror::Error;

use crate::model::EnvironmentState;

pub trait EnvironmentStateStore {
    fn load(&self) -> Result<EnvironmentState, EnvironmentStateStoreError>;
    fn save(&self, state: &EnvironmentState) -> Result<(), EnvironmentStateStoreError>;
}

#[derive(Debug, Error)]
pub enum EnvironmentStateStoreError {
    #[error("unknown environment-state setting: {0}")]
    UnknownSetting(String),
    #[error("invalid environment-state setting: {0}")]
    InvalidSettingValue(String),
    #[error("failed to read settings file: {path}")]
    SettingsReadFailed {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },
    #[error("failed to save environment settings")]
    SettingsSaveFailed {
        #[source]
        source: anyhow::Error,
    },
    #[error("failed to load environment state")]
    StateLoadFailed {
        #[source]
        source: anyhow::Error,
    },
}
