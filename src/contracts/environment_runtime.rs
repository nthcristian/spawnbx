use thiserror::Error;

use crate::model::{CommandExecutor, EnvironmentState};

pub trait EnvironmentRuntime {
    fn attach(&self, environment_state: &EnvironmentState) -> Result<(), EnvironmentRuntimeError>;
    fn executor(&self) -> CommandExecutor<'_>;
    fn ensure_running(&self) -> Result<(), EnvironmentRuntimeError>;
    fn remove(&self) -> Result<(), EnvironmentRuntimeError>;
}

#[derive(Debug, Error)]
pub enum EnvironmentRuntimeError {
    #[error("failed to attach to the environment: {0}")]
    AttachmentFailed(String),
    #[error("failed to execute inside the environment: {0}")]
    CommandExecutionFailed(String),
    #[error("environment runtime failed")]
    RuntimeFailure {
        #[from]
        source: anyhow::Error,
    },
}
