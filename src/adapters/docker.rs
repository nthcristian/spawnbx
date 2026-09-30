use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
    process::{Command, Output},
};

use anyhow::Context;

use crate::{
    contracts::{EnvironmentRuntime, EnvironmentRuntimeError},
    model::{CommandExecutor, CommandOutput, EnvironmentState},
};

#[cfg(not(debug_assertions))]
const DEFAULT_IMAGE: &str = "ghcr.io/nthcristian/spawnbx:latest";

#[cfg(debug_assertions)]
const DEFAULT_IMAGE: &str = "spawnbx:latest";

pub struct DockerEnvironment {
    environment_state: EnvironmentState,
    container_name: String,
}

impl EnvironmentRuntime for DockerEnvironment {
    fn attach(&self) -> Result<(), EnvironmentRuntimeError> {
        tracing::info!(container = self.container_name(), "attaching to container");
        let status = Command::new("docker")
            .args([
                "exec",
                "--interactive",
                "--tty",
                "--user",
                &format!(
                    "{}:{}",
                    self.environment_state.host_uid, self.environment_state.host_gid
                ),
                self.container_name(),
                &self.environment_state.shell_program,
            ])
            .status()
            .with_context(|| {
                format!(
                    "could not attach to Docker container {}",
                    self.container_name()
                )
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(EnvironmentRuntimeError::AttachmentFailed(format!(
                "docker exec for {} exited with {status}",
                self.container_name()
            )))
        }
    }

    fn executor(&self) -> CommandExecutor<'_> {
        Box::new(move |command| {
            let output = Command::new("docker")
                .args(["exec", self.container_name()])
                .args(command)
                .output()
                .with_context(|| {
                    format!(
                        "could not execute in Docker container {}",
                        self.container_name()
                    )
                })
                .map_err(EnvironmentRuntimeError::from)?;

            Ok(CommandOutput {
                stdout: String::from_utf8(output.stdout)
                    .with_context(|| {
                        format!(
                            "could not decode stdout from Docker container {} as UTF-8",
                            self.container_name()
                        )
                    })
                    .map_err(EnvironmentRuntimeError::from)?,
                stderr: String::from_utf8(output.stderr)
                    .with_context(|| {
                        format!(
                            "could not decode stderr from Docker container {} as UTF-8",
                            self.container_name()
                        )
                    })
                    .map_err(EnvironmentRuntimeError::from)?,
                exit_code: output.status.code(),
            })
        })
    }

    fn ensure_running(&self) -> Result<(), EnvironmentRuntimeError> {
        let inspect = self.inspect()?;

        // Keep this after a completed inspect, including a nonzero inspect result.
        let home = Path::new(&self.environment_state.workspace_root).join(".spawnbx/home");
        fs::create_dir_all(&home)
            .with_context(|| format!("could not create home directory {}", home.display()))?;

        if inspect.status.success() {
            return if inspect.stdout.trim_ascii() == b"true" {
                Ok(())
            } else {
                self.start()
            };
        }

        if !String::from_utf8_lossy(&inspect.stderr).contains("No such container") {
            return Err(anyhow::anyhow!(
                "could not inspect Docker container {} ({}): {}",
                self.container_name(),
                inspect.status,
                String::from_utf8_lossy(&inspect.stderr).trim()
            )
            .into());
        }

        self.create(&home)
    }

    fn remove(&self) -> Result<(), EnvironmentRuntimeError> {
        tracing::info!(container = self.container_name(), "removing container");
        self.run_docker_successfully(&["rm", self.container_name()], "remove")
    }
}

impl DockerEnvironment {
    pub fn container_name(&self) -> &str {
        &self.container_name
    }

    fn inspect(&self) -> Result<Output, EnvironmentRuntimeError> {
        self.run_docker(
            &[
                "container",
                "inspect",
                "--format",
                "{{.State.Running}}",
                self.container_name(),
            ],
            "inspect",
        )
    }

    fn start(&self) -> Result<(), EnvironmentRuntimeError> {
        tracing::info!(container = self.container_name(), "starting container");
        self.run_docker_successfully(&["container", "start", self.container_name()], "start")
    }

    fn create(&self, home: &Path) -> Result<(), EnvironmentRuntimeError> {
        tracing::info!(container = self.container_name(), "creating container");
        let workspace_mount = format!("{}:/workspace", self.environment_state.workspace_root);
        let home_mount = format!("{}:/home/spawnbx", home.display());

        self.run_docker_successfully(
            &[
                "run",
                "--detach",
                "--name",
                self.container_name(),
                "--workdir",
                "/workspace",
                "--env",
                "HOME=/home/spawnbx",
                "--volume",
                &workspace_mount,
                "--volume",
                &home_mount,
                DEFAULT_IMAGE,
                "sleep",
                "infinity",
            ],
            "create",
        )
    }

    fn run_docker(
        &self,
        args: &[&str],
        operation: &str,
    ) -> Result<Output, EnvironmentRuntimeError> {
        Command::new("docker")
            .args(args)
            .output()
            .with_context(|| {
                format!(
                    "could not {operation} Docker container {}",
                    self.container_name()
                )
            })
            .map_err(EnvironmentRuntimeError::from)
    }

    fn run_docker_successfully(
        &self,
        args: &[&str],
        operation: &str,
    ) -> Result<(), EnvironmentRuntimeError> {
        let output = self.run_docker(args, operation)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "could not {operation} Docker container {} ({}): {}",
                self.container_name(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into())
        }
    }
}

impl TryFrom<&EnvironmentState> for DockerEnvironment {
    type Error = EnvironmentRuntimeError;

    fn try_from(value: &EnvironmentState) -> Result<Self, Self::Error> {
        Ok(Self {
            environment_state: value.clone(),
            container_name: format!(
                "{}-{}",
                value.container_name_prefix,
                workspace_hash(&value.workspace_root)
            ),
        })
    }
}

impl Drop for DockerEnvironment {
    fn drop(&mut self) {
        tracing::info!(container = self.container_name(), "stopping container");
        if let Err(error) =
            self.run_docker_successfully(&["container", "stop", self.container_name()], "stop")
        {
            tracing::debug!(container = self.container_name(), error = ?error, "ignoring container cleanup failure");
        }
    }
}

fn workspace_hash(source: &str) -> String {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);

    hasher.finish().to_be_bytes()[..4]
        .iter()
        .map(|b| format!("{:02}", b % 99 + 1))
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/docker.rs"]
mod tests;
