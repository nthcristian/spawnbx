use std::collections::BTreeSet;

use anyhow::Context;
use serde_json::Value;

use crate::{
    contracts::{EnvironmentConfigurationError, EnvironmentConfigurator, execute_checked},
    model::{
        CommandExecutor, CommandInvocation, CommandOperation, CommandOutput, EnvironmentState,
    },
};

const NIX_SYSTEM_PROFILE: &str = "/nix/var/nix/profiles/default";
const NIX_PATH_SETUP_SCRIPT: &str = r#"
mkdir -p /etc/profile.d
cat > /etc/profile.d/spawnbx-nix.sh <<'EOF'
case ":$PATH:" in
    *":/nix/var/nix/profiles/default/bin:"*) ;;
    *) export PATH="/nix/var/nix/profiles/default/bin:$PATH" ;;
esac
EOF
chmod 0644 /etc/profile.d/spawnbx-nix.sh
"#;

pub struct NixConfigurator;

impl EnvironmentConfigurator for NixConfigurator {
    fn configure(
        executor: &CommandExecutor,
        environment_state: &EnvironmentState,
        invocation: &CommandInvocation,
    ) -> Result<(), EnvironmentConfigurationError> {
        execute_checked(
            executor,
            &["sh", "-eu", "-c", NIX_PATH_SETUP_SCRIPT],
            "configure the system Nix PATH",
        )?;
        let installed_package_names = profile_packages(execute_checked(
            executor,
            &[
                "nix",
                "profile",
                "list",
                "--json",
                "--profile",
                NIX_SYSTEM_PROFILE,
            ],
            "inspect the system Nix profile",
        )?)?;

        if let CommandOperation::Update { package_names } = &invocation.operation {
            update_packages(executor, package_names, &installed_package_names)?;
        }

        let missing_packages = environment_state
            .package_names
            .iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|package| !installed_package_names.contains(*package))
            .map(|package| format!("nixpkgs#{package}"))
            .collect::<Vec<_>>();

        if missing_packages.is_empty() {
            return Ok(());
        }

        let mut command = vec!["nix", "profile", "install", "--profile", NIX_SYSTEM_PROFILE];
        command.extend(missing_packages.iter().map(String::as_str));
        tracing::info!(
            package_count = missing_packages.len(),
            "Installing Nix packages"
        );
        execute_checked(executor, &command, "install Nix packages")?;

        Ok(())
    }
}

fn update_packages(
    executor: &CommandExecutor,
    package_names: &[String],
    installed_package_names: &BTreeSet<String>,
) -> Result<(), EnvironmentConfigurationError> {
    if package_names.is_empty() {
        tracing::info!("Updating all Nix packages");
        execute_checked(
            executor,
            &[
                "nix",
                "profile",
                "upgrade",
                "'.*'",
                "--profile",
                NIX_SYSTEM_PROFILE,
            ],
            "update all Nix packages",
        )?;
    } else {
        for package in package_names {
            if !installed_package_names.contains(package) {
                return Err(anyhow::anyhow!("The package {package} isn't installed").into());
            }

            tracing::info!(package, "Updating Nix package");
            execute_checked(
                executor,
                &[
                    "nix",
                    "profile",
                    "upgrade",
                    package,
                    "--profile",
                    NIX_SYSTEM_PROFILE,
                ],
                &format!("update Nix package {package}"),
            )?;
        }
    }
    Ok(())
}

fn profile_packages(
    output: CommandOutput,
) -> Result<BTreeSet<String>, EnvironmentConfigurationError> {
    let profile: Value =
        serde_json::from_str(&output.stdout).context("failed to parse the system Nix profile")?;
    let elements = profile["elements"].as_object().ok_or_else(|| {
        EnvironmentConfigurationError::ConfigurationFailed {
            source: anyhow::anyhow!("Nix returned a profile without elements"),
        }
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
#[path = "../../tests/unit/plugins.rs"]
mod tests;
