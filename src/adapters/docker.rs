use std::{
    hash::{DefaultHasher, Hash, Hasher},
    process::Command,
};

use crate::{
    contracts::{Environment, EnvironmentError},
    model::DesiredContainerState,
};

pub struct Docker {
    desired_state: DesiredContainerState,
    hash: String,
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
                &self.container_name(),
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
            let user = format!(
                "{}:{}",
                self.desired_state.host_uid, self.desired_state.host_gid
            );
            let container_name = self.container_name();
            let output = Command::new("docker")
                .args(["exec", "--user", &user, &container_name])
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
    fn container_name(&self) -> String {
        format!("{}-{}", self.desired_state.container_name_prefix, self.hash)
    }
}

impl TryFrom<&DesiredContainerState> for Docker {
    type Error = EnvironmentError;

    fn try_from(value: &DesiredContainerState) -> Result<Self, Self::Error> {
        let hash = hash_string(value.workspace_root.clone())?;

        println!("container name: {}-{}", value.container_name_prefix, &hash);

        Ok(Self {
            desired_state: value.clone(),
            hash,
        })
    }
}

fn runtime_error(source: impl Into<anyhow::Error>) -> EnvironmentError {
    EnvironmentError::RuntimeError {
        source: source.into(),
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
