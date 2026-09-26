use crate::{
    contracts::{EnvironmentPlugin, PluginError},
    model::{DesiredContainerState, Executor},
};

pub struct UserPlugin {}

impl EnvironmentPlugin for UserPlugin {
    fn plug(executor: &Executor, desired_state: &DesiredContainerState) -> Result<(), PluginError> {
        let uid = desired_state.host_uid.to_string();
        let gid = desired_state.host_gid.to_string();
        let output = executor(&[
            "sh",
            "-eu",
            "-c",
            r#"
username="$1"
uid="$2"
gid="$3"

if id -- "$username" >/dev/null 2>&1; then
    [ "$(id -u -- "$username")" = "$uid" ]
    [ "$(id -g -- "$username")" = "$gid" ]
else
    if ! getent group "$gid" >/dev/null; then
        groupadd --gid "$gid" -- "$username"
    fi

    useradd --uid "$uid" --gid "$gid" --home-dir /home/spawnbx \
        --create-home --shell /bin/bash -- "$username"
fi

usermod --append --groups wheel -- "$username"
printf '%s ALL=(ALL) NOPASSWD: ALL\n' "$username" > /etc/sudoers.d/spawnbx
chmod 0440 /etc/sudoers.d/spawnbx
"#,
            "--",
            &desired_state.host_username,
            &uid,
            &gid,
        ])
        .map_err(|source| PluginError::RuntimeError { source })?;

        if output.exit_code == Some(0) {
            Ok(())
        } else {
            Err(PluginError::RuntimeError {
                source: anyhow::anyhow!("failed to configure container user: {}", output.stderr),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use crate::{
        contracts::EnvironmentPlugin,
        model::{DesiredContainerState, ExecutionOutput, Executor},
    };

    use super::UserPlugin;

    #[test]
    fn passes_the_host_identity_to_the_setup_command() {
        let command = RefCell::new(Vec::new());
        let executor: Executor = Box::new(|arguments| {
            command
                .borrow_mut()
                .extend(arguments.iter().map(|argument| argument.to_string()));
            Ok(ExecutionOutput {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: Some(0),
            })
        });
        let state = DesiredContainerState {
            container_name_prefix: String::new(),
            workspace_root: String::new(),
            package_names: Vec::new(),
            shell_program: String::new(),
            host_username: "developer".into(),
            host_uid: 1000,
            host_gid: 1001,
        };

        UserPlugin::plug(&executor, &state);

        let command = command.borrow();
        assert_eq!(
            &command[..3],
            ["sh", "-eu", "-c"],
            "setup must run through a non-interactive shell"
        );
        assert!(command[3].contains("useradd"));
        assert_eq!(
            &command[4..],
            ["--", "developer", "1000", "1001"],
            "setup must receive the host identity as positional arguments"
        );
    }
}
