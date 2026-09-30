use std::{
    path::Path,
    process::{Command, Output},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{
    contracts::{EnvironmentStateStore, EnvironmentStateStoreError},
    model::EnvironmentState,
};

const SETTINGS_FILE: &str = ".spawnbx.yml";

pub struct ProjectSettingsStore;

impl EnvironmentStateStore for ProjectSettingsStore {
    fn load(&self) -> Result<EnvironmentState, EnvironmentStateStoreError> {
        let settings = load_settings(Path::new(SETTINGS_FILE))?;
        let workspace_root = std::env::current_dir()
            .context("resolve current directory while loading settings")
            .map_err(load_error)?;
        let host_user = current_host_user()?;

        assemble_environment_state(settings, &workspace_root, host_user)
    }

    fn save(&self, state: &EnvironmentState) -> Result<(), EnvironmentStateStoreError> {
        let yaml = serialize_settings(state)?;
        let workspace_root = std::env::current_dir()
            .context("resolve current directory while saving settings")
            .map_err(load_error)?;
        save_settings(&workspace_root.join(SETTINGS_FILE), &yaml)
    }
}

fn load_settings(path: &Path) -> Result<ProjectSettings, EnvironmentStateStoreError> {
    let yaml = std::fs::read_to_string(path)
        .with_context(|| format!("read project settings at {}", path.display()))
        .map_err(|source| EnvironmentStateStoreError::SettingsReadFailed {
            path: path.to_owned(),
            source,
        })?;

    serde_yaml::from_str(&yaml)
        .with_context(|| format!("parse project settings at {}", path.display()))
        .map_err(load_error)
}

fn assemble_environment_state(
    settings: ProjectSettings,
    workspace_root: &Path,
    host: HostIdentity,
) -> Result<EnvironmentState, EnvironmentStateStoreError> {
    let container_name_prefix = match settings.container_name_prefix {
        Some(name) => name,
        None => workspace_name(workspace_root)?,
    };

    Ok(EnvironmentState {
        container_name_prefix,
        shell_program: settings.shell_program.unwrap_or_else(|| "bash".into()),
        package_names: settings.package_names.unwrap_or_default(),
        workspace_root: workspace_root.to_string_lossy().into_owned(),
        host_username: host.username,
        host_uid: host.uid,
        host_gid: host.gid,
    })
}

fn serialize_settings(state: &EnvironmentState) -> Result<String, EnvironmentStateStoreError> {
    let settings = ProjectSettings {
        container_name_prefix: Some(state.container_name_prefix.clone()),
        shell_program: Some(state.shell_program.clone()),
        package_names: Some(state.package_names.clone()),
    };

    serde_yaml::to_string(&settings)
        .context("serialize project settings")
        .map_err(save_error)
}

fn save_settings(path: &Path, yaml: &str) -> Result<(), EnvironmentStateStoreError> {
    tracing::info!(path = %path.display(), "Saving project settings");
    std::fs::write(path, yaml)
        .with_context(|| format!("write project settings at {}", path.display()))
        .map_err(save_error)
}

fn workspace_name(path: &Path) -> Result<String, EnvironmentStateStoreError> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| load_error(anyhow::anyhow!("workspace root has no directory name")))
}

fn current_host_user() -> Result<HostIdentity, EnvironmentStateStoreError> {
    Ok(HostIdentity {
        username: read_identity_value("-un")?,
        uid: read_identity_value("-u")?
            .parse()
            .context("parse host user ID from id -u")
            .map_err(load_error)?,
        gid: read_identity_value("-g")?
            .parse()
            .context("parse host group ID from id -g")
            .map_err(load_error)?,
    })
}

fn read_identity_value(arg: &str) -> Result<String, EnvironmentStateStoreError> {
    let output = Command::new("id")
        .arg(arg)
        .output()
        .with_context(|| format!("run id {arg} to look up host identity"))
        .map_err(load_error)?;
    decode_identity_output(arg, output)
}

fn decode_identity_output(arg: &str, output: Output) -> Result<String, EnvironmentStateStoreError> {
    if !output.status.success() {
        return Err(load_error(anyhow::anyhow!(
            "id {arg} failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let value = String::from_utf8(output.stdout)
        .with_context(|| format!("decode host identity from id {arg}"))
        .map_err(load_error)?
        .trim()
        .to_owned();

    if value.is_empty() {
        return Err(load_error(anyhow::anyhow!(
            "id {arg} returned an empty value"
        )));
    }

    Ok(value)
}

fn load_error(source: impl Into<anyhow::Error>) -> EnvironmentStateStoreError {
    EnvironmentStateStoreError::StateLoadFailed {
        source: source.into(),
    }
}

fn save_error(source: impl Into<anyhow::Error>) -> EnvironmentStateStoreError {
    EnvironmentStateStoreError::SettingsSaveFailed {
        source: source.into(),
    }
}

struct HostIdentity {
    username: String,
    uid: u32,
    gid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectSettings {
    #[serde(rename = "name")]
    container_name_prefix: Option<String>,

    #[serde(rename = "shell")]
    shell_program: Option<String>,

    #[serde(rename = "packages")]
    package_names: Option<Vec<String>>,
}

#[cfg(test)]
#[path = "../../tests/unit/settings.rs"]
mod tests;
