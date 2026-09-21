use std::path::PathBuf;

use super::ports::WorkspaceAdapter;
use super::types::{
    EffectiveConfig, IntegrationRequests, Invocation, NetworkMode, PackageList, PackageName,
    ProjectConfig, ProjectContext, ProjectRoot, ShellName,
};

pub(crate) trait ProjectModule {
    fn resolve(
        &mut self,
        start: PathBuf,
        workspace: &mut dyn WorkspaceAdapter,
        create_state: bool,
    ) -> Result<ProjectContext, ProjectError>;

    fn merge(
        &mut self,
        config: ProjectConfig,
        invocation: &Invocation,
    ) -> Result<EffectiveConfig, ProjectError>;

    fn save(
        &mut self,
        root: &ProjectRoot,
        config: &EffectiveConfig,
        workspace: &mut dyn WorkspaceAdapter,
    ) -> Result<(), ProjectError>;
}

pub(crate) struct ProjectResolver;

impl ProjectResolver {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl ProjectModule for ProjectResolver {
    fn resolve(
        &mut self,
        start: PathBuf,
        workspace: &mut dyn WorkspaceAdapter,
        create_state: bool,
    ) -> Result<ProjectContext, ProjectError> {
        let root = workspace.discover(start).map_err(ProjectError::adapter)?;
        let config_path = root.canonical_path.join(".spawnbx.yml");
        let config = workspace
            .read_config(&root)
            .map_err(ProjectError::adapter)?
            .unwrap_or_default();
        let state_paths = workspace
            .ensure_state(&root, create_state)
            .map_err(ProjectError::adapter)?;

        Ok(ProjectContext {
            root,
            config_path: config_path.is_file().then_some(config_path),
            config,
            state_paths,
        })
    }

    fn merge(
        &mut self,
        config: ProjectConfig,
        invocation: &Invocation,
    ) -> Result<EffectiveConfig, ProjectError> {
        let shell = invocation
            .overrides
            .shell
            .clone()
            .or(config.shell)
            .unwrap_or_else(|| "bash".to_owned());
        validate_name(&shell)?;

        let package_values = invocation
            .overrides
            .packages
            .clone()
            .or(config.packages)
            .unwrap_or_default();
        let packages = package_values
            .into_iter()
            .map(PackageName::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        let integrations = IntegrationRequests {
            x11: invocation.overrides.x11.or(config.x11).unwrap_or(false),
            wayland: invocation
                .overrides
                .wayland
                .or(config.wayland)
                .unwrap_or(false),
            pipewire: invocation
                .overrides
                .pipewire
                .or(config.pipewire)
                .unwrap_or(false),
            gpu: invocation.overrides.gpu.or(config.gpu).unwrap_or(false),
        };
        let network = invocation
            .overrides
            .network
            .clone()
            .or(config.network)
            .unwrap_or(NetworkMode::Bridge);

        Ok(EffectiveConfig {
            name: config.name,
            shell: ShellName { executable: shell },
            packages: PackageList { values: packages },
            integrations,
            network,
        })
    }

    fn save(
        &mut self,
        root: &ProjectRoot,
        config: &EffectiveConfig,
        workspace: &mut dyn WorkspaceAdapter,
    ) -> Result<(), ProjectError> {
        let project_config = ProjectConfig {
            name: config.name.clone(),
            shell: Some(config.shell.executable.clone()),
            packages: Some(
                config
                    .packages
                    .values
                    .iter()
                    .map(|package| package.attribute_path.clone())
                    .collect(),
            ),
            x11: Some(config.integrations.x11),
            wayland: Some(config.integrations.wayland),
            pipewire: Some(config.integrations.pipewire),
            gpu: Some(config.integrations.gpu),
            network: Some(config.network.clone()),
        };
        workspace
            .write_config(root, &project_config)
            .map_err(ProjectError::adapter)
    }
}

fn validate_name(value: &str) -> Result<(), ProjectError> {
    if value.is_empty()
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(char::is_whitespace)
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || "._+-".contains(character)))
    {
        return Err(ProjectError::message(
            "shell must be one executable name without arguments",
        ));
    }
    Ok(())
}

impl TryFrom<String> for PackageName {
    type Error = ProjectError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.split('.').any(|segment| {
                segment.is_empty()
                    || !segment.chars().all(|character| {
                        character.is_ascii_alphanumeric() || "_-".contains(character)
                    })
                    || segment
                        .chars()
                        .next()
                        .is_some_and(|character| character.is_ascii_digit())
            })
        {
            return Err(ProjectError::message("invalid Nix package attribute"));
        }
        Ok(Self {
            attribute_path: value,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProjectError {
    pub(crate) message: String,
}

impl ProjectError {
    fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn adapter(error: super::ports::AdapterError) -> Self {
        Self::message(format!("{}: {}", error.category, error.message))
    }
}

#[cfg(test)]
mod tests {
    use super::{ProjectModule, ProjectResolver};
    use crate::reconcile::{Invocation, Operation, Overrides, ProjectConfig};

    #[test]
    fn merges_defaults_without_project_configuration() {
        let mut resolver = ProjectResolver::new();
        let invocation = Invocation {
            cwd: ".".into(),
            operation: Operation::Attach,
            overrides: Overrides::default(),
            save: false,
            allow_missing_integrations: false,
        };

        let config = resolver
            .merge(ProjectConfig::default(), &invocation)
            .expect("defaults are valid");

        assert_eq!(config.shell.executable, "bash");
        assert!(config.packages.values.is_empty());
        assert!(matches!(
            config.network,
            crate::reconcile::NetworkMode::Bridge
        ));
    }

    #[test]
    fn command_line_packages_replace_project_packages() {
        let mut resolver = ProjectResolver::new();
        let mut overrides = Overrides::default();
        overrides.packages = Some(vec!["ripgrep".to_owned()]);
        let invocation = Invocation {
            cwd: ".".into(),
            operation: Operation::Attach,
            overrides,
            save: false,
            allow_missing_integrations: false,
        };
        let config = ProjectConfig {
            packages: Some(vec!["fish".to_owned()]),
            ..ProjectConfig::default()
        };

        let effective = resolver.merge(config, &invocation).expect("valid merge");

        assert_eq!(effective.packages.values[0].attribute_path, "ripgrep");
    }
}
