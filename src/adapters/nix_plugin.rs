use std::collections::BTreeSet;

use serde_json::Value;

use crate::{
    contracts::{EnvironmentPlugin, PluginError},
    model::{DesiredContainerState, ExecutionOutput, Executor},
};

const SYSTEM_PROFILE: &str = "/nix/var/nix/profiles/default";
const SYSTEM_PATH_SCRIPT: &str = r#"
mkdir -p /etc/profile.d
cat > /etc/profile.d/spawnbx-nix.sh <<'EOF'
case ":$PATH:" in
    *":/nix/var/nix/profiles/default/bin:"*) ;;
    *) export PATH="/nix/var/nix/profiles/default/bin:$PATH" ;;
esac
EOF
chmod 0644 /etc/profile.d/spawnbx-nix.sh
"#;

pub struct NixPlugin {}

impl EnvironmentPlugin for NixPlugin {
    fn plug(executor: &Executor, desired_state: &DesiredContainerState) -> Result<(), PluginError> {
        execute(
            executor,
            &["sh", "-eu", "-c", SYSTEM_PATH_SCRIPT],
            "configure the system Nix PATH",
        )?;
        let installed = installed_packages(execute(
            executor,
            &[
                "nix",
                "profile",
                "list",
                "--json",
                "--profile",
                SYSTEM_PROFILE,
            ],
            "inspect the system Nix profile",
        )?)?;
        let missing = desired_state
            .package_names
            .iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|package| !installed.contains(*package))
            .map(|package| format!("nixpkgs#{package}"))
            .collect::<Vec<_>>();

        if missing.is_empty() {
            return Ok(());
        }

        let mut command = vec!["nix", "profile", "install", "--profile", SYSTEM_PROFILE];
        command.extend(missing.iter().map(String::as_str));
        execute(executor, &command, "install Nix packages")?;

        Ok(())
    }
}

fn execute(
    executor: &Executor,
    command: &[&str],
    operation: &str,
) -> Result<ExecutionOutput, PluginError> {
    let output = executor(command).map_err(|source| PluginError::RuntimeError { source })?;

    if output.exit_code == Some(0) {
        Ok(output)
    } else {
        Err(PluginError::RuntimeError {
            source: anyhow::anyhow!("failed to {operation}: {}", output.stderr),
        })
    }
}

fn installed_packages(output: ExecutionOutput) -> Result<BTreeSet<String>, PluginError> {
    let profile: Value =
        serde_json::from_str(&output.stdout).map_err(|source| PluginError::RuntimeError {
            source: source.into(),
        })?;
    let elements = profile["elements"]
        .as_object()
        .ok_or_else(|| PluginError::RuntimeError {
            source: anyhow::anyhow!("Nix returned a profile without elements"),
        })?;

    Ok(elements
        .values()
        .filter(|element| element["active"].as_bool().unwrap_or(true))
        .filter_map(|element| element["attrPath"].as_str())
        .map(|attribute| {
            attribute
                .strip_prefix("packages.x86_64-linux.")
                .or_else(|| attribute.strip_prefix("legacyPackages.x86_64-linux."))
                .unwrap_or(attribute)
                .to_owned()
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use crate::model::ExecutionOutput;

    use super::installed_packages;

    #[test]
    fn extracts_package_attributes_from_a_nix_profile() {
        let installed = installed_packages(ExecutionOutput {
            stdout:
                r#"{"elements":{"0":{"active":true,"attrPath":"packages.x86_64-linux.ripgrep"}}}"#
                    .into(),
            stderr: String::new(),
            exit_code: Some(0),
        })
        .unwrap();

        assert!(installed.contains("ripgrep"));
    }
}
