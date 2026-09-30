use anyhow::Context;
use thiserror::Error;

use crate::model::{CommandExecutor, CommandInvocation, CommandOutput, EnvironmentState};

pub trait EnvironmentConfigurator {
    fn configure(
        executor: &CommandExecutor,
        environment_state: &EnvironmentState,
        invocation: &CommandInvocation,
    ) -> Result<(), EnvironmentConfigurationError>;
}

#[derive(Debug, Error)]
pub enum EnvironmentConfigurationError {
    #[error("failed to configure environment")]
    ConfigurationFailed {
        #[from]
        source: anyhow::Error,
    },
}

pub(crate) fn execute_checked(
    executor: &CommandExecutor,
    command: &[&str],
    operation: &str,
) -> Result<CommandOutput, EnvironmentConfigurationError> {
    let output = executor(command).with_context(|| format!("failed to {operation}"))?;
    if output.exit_code == Some(0) {
        return Ok(output);
    }

    let status = output.exit_code.map_or_else(
        || "no exit code".to_owned(),
        |code| format!("exit code {code}"),
    );
    Err(anyhow::anyhow!("failed to {operation}: {status}; stderr: {}", output.stderr).into())
}
