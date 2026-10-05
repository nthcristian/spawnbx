use std::collections::HashSet;

use thiserror::Error;

use crate::{
    adapters::{
        ClapInvocationParser, DockerEnvironment, NixConfigurator, ProjectSettingsStore,
        UserConfigurator,
    },
    contracts::{
        EnvironmentConfigurator, EnvironmentRuntime, EnvironmentStateStore,
        EnvironmentStateStoreError, InvocationParseError, InvocationParser,
    },
    model::{CommandInvocation, CommandOperation, EnvironmentState},
};

pub mod adapters;
pub mod contracts;
pub mod model;

pub fn run() -> Result<(), ApplicationError> {
    let invocation = ClapInvocationParser.parse()?;
    let state = resolve_environment_state(&invocation, &ProjectSettingsStore)?;
    let environment = DockerEnvironment::try_from(&state).map_err(ApplicationError::environment)?;

    if matches!(
        invocation.operation,
        CommandOperation::Remove
            | CommandOperation::Recreate {
                allow_missing_integrations: _
            }
    ) {
        environment
            .remove()
            .map_err(ApplicationError::environment)?;
    }

    if matches!(
        invocation.operation,
        CommandOperation::Remove | CommandOperation::Stop
    ) {
        return Ok(());
    }

    environment
        .ensure_running()
        .map_err(ApplicationError::environment)?;

    let executor = environment.executor();
    tracing::info!("Configuring environment");
    UserConfigurator::configure(&executor, &state, &invocation)
        .map_err(ApplicationError::environment)?;
    NixConfigurator::configure(&executor, &state, &invocation)
        .map_err(ApplicationError::environment)?;

    if matches!(invocation.operation, CommandOperation::Update { .. }) {
        return Ok(());
    }

    environment
        .attach()
        .map_err(ApplicationError::environment)?;

    Ok(())
}

fn resolve_environment_state(
    invocation: &CommandInvocation,
    store: &impl EnvironmentStateStore,
) -> Result<EnvironmentState, ApplicationError> {
    let mut state = store.load()?;

    if let Some(shell) = &invocation.requested_shell {
        state.shell_program.clone_from(shell);
    }
    state.wayland |= invocation.wayland;
    state.pipewire |= invocation.pipewire;
    state.gpu |= invocation.gpu;

    let packages: HashSet<_> = invocation
        .additional_package_names
        .iter()
        .cloned()
        .chain(state.package_names)
        .collect();
    state.package_names = packages.into_iter().collect();

    if invocation.save_settings {
        store.save(&state)?;
    }

    Ok(state)
}

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("failed to parse command invocation")]
    InvocationParseFailed {
        #[from]
        source: InvocationParseError,
    },
    #[error("failed to resolve environment state")]
    StateResolutionFailed {
        #[from]
        source: EnvironmentStateStoreError,
    },
    #[error("failed to run environment")]
    EnvironmentFailed {
        #[source]
        source: anyhow::Error,
    },
}

impl ApplicationError {
    fn environment(source: impl Into<anyhow::Error>) -> Self {
        Self::EnvironmentFailed {
            source: source.into(),
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/bootstrap.rs"]
mod tests;
