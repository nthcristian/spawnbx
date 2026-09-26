use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
    process::{Command, Output},
};

use crate::{
    contracts::{Environment, EnvironmentError},
    model::DesiredContainerState,
};

pub struct Docker {
    desired_state: DesiredContainerState,
    fullname: String,
}

impl Environment for Docker {
    fn attach(&self) -> Result<(), EnvironmentError> {
        let status = Command::new("docker")
            .args([
                "exec",
                "--interactive",
                "--tty",
                "--user",
                &format!(
                    "{}:{}",
                    self.desired_state.host_uid, self.desired_state.host_gid
                ),
                self.container_name(),
                &self.desired_state.shell_program,
            ])
            .status()
            .map_err(runtime_error)?;

        if status.success() {
            Ok(())
        } else {
            Err(EnvironmentError::AttachError(format!(
                "docker exec exited with {status}"
            )))
        }
    }

    fn get_executor(
        &self,
    ) -> impl Fn(&[&str]) -> Result<crate::model::ExecutionOutput, EnvironmentError> + '_ {
        move |command| {
            let container_name = self.container_name();
            let output = Command::new("docker")
                .args(["exec", container_name])
                .args(command)
                .output()
                .map_err(runtime_error)?;

            Ok(crate::model::ExecutionOutput {
                stdout: String::from_utf8(output.stdout).map_err(runtime_error)?,
                stderr: String::from_utf8(output.stderr).map_err(runtime_error)?,
                exit_code: output.status.code(),
            })
        }
    }
}

impl Docker {
    fn container_name(&self) -> &str {
        &self.fullname
    }

    fn ensure_running(&self) -> Result<(), EnvironmentError> {
        let inspect = docker(&[
            "container",
            "inspect",
            "--format",
            "{{.State.Running}}",
            self.container_name(),
        ])?;

        if inspect.status.success() {
            return if inspect.stdout.trim_ascii() == b"true" {
                Ok(())
            } else {
                docker_success(&["container", "start", self.container_name()], "start")
            };
        }

        if !String::from_utf8_lossy(&inspect.stderr).contains("No such container") {
            return Err(runtime_error(anyhow::anyhow!(
                "could not inspect Docker container {}: {}",
                self.container_name(),
                String::from_utf8_lossy(&inspect.stderr).trim()
            )));
        }

        let state_root = PathBuf::from(&self.desired_state.workspace_root).join(".spawnbx");
        fs::create_dir_all(state_root.join("home")).map_err(runtime_error)?;
        fs::create_dir_all(state_root.join("nix")).map_err(runtime_error)?;

        let workspace_mount = format!("{}:/workspace", self.desired_state.workspace_root);
        let home_mount = format!("{}:/home/spawnbx", state_root.join("home").display());
        let nix_mount = format!(
            "{}:/workspace/.spawnbx/nix",
            state_root.join("nix").display()
        );
        docker_success(
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
                "--volume",
                &nix_mount,
                "ghcr.io/nthcristian/spawnbx:latest",
                "sleep",
                "infinity",
            ],
            "create",
        )
    }
}

impl TryFrom<&DesiredContainerState> for Docker {
    type Error = EnvironmentError;

    fn try_from(value: &DesiredContainerState) -> Result<Self, Self::Error> {
        let hash = hash_string(value.workspace_root.clone())?;

        let docker = Self {
            desired_state: value.clone(),
            fullname: format!("{}-{hash}", value.container_name_prefix),
        };

        docker.ensure_running()?;

        Ok(docker)
    }
}

impl Drop for Docker {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["container", "stop", &self.fullname])
            .output();
    }
}

fn runtime_error(source: impl Into<anyhow::Error>) -> EnvironmentError {
    EnvironmentError::RuntimeError {
        source: source.into(),
    }
}

fn docker(args: &[&str]) -> Result<Output, EnvironmentError> {
    Command::new("docker")
        .args(args)
        .output()
        .map_err(runtime_error)
}

fn docker_success(args: &[&str], operation: &str) -> Result<(), EnvironmentError> {
    let output = docker(args)?;

    if output.status.success() {
        Ok(())
    } else {
        Err(runtime_error(anyhow::anyhow!(
            "could not {operation} Docker container: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn hash_string(source: String) -> Result<String, EnvironmentError> {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);

    let v: Vec<String> = hasher.finish().to_be_bytes()[..4]
        .to_vec()
        .iter()
        .map(|b| format!("{:02}", b % 99 + 1))
        .collect();

    Ok(v.join(""))
}
