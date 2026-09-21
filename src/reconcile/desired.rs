use sha2::{Digest, Sha256};

use super::types::{
    CapabilityGrants, ContainerName, ContainerSpec, DesiredState, EffectiveConfig, HostFacts,
    ImageRef, ManagedLabels, Mount, MountSet, PackageIntent, ProjectContext, ProjectRoot, SpecHash,
};

pub(crate) trait DesiredStateModule {
    fn build(
        &mut self,
        project: &ProjectContext,
        config: &EffectiveConfig,
        facts: &HostFacts,
        grants: &CapabilityGrants,
    ) -> Result<DesiredState, DesiredStateError>;
}

pub(crate) struct DesiredStateBuilder;

impl DesiredStateBuilder {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl DesiredStateModule for DesiredStateBuilder {
    fn build(
        &mut self,
        project: &ProjectContext,
        config: &EffectiveConfig,
        facts: &HostFacts,
        grants: &CapabilityGrants,
    ) -> Result<DesiredState, DesiredStateError> {
        let base_name = config
            .name
            .as_deref()
            .or_else(|| {
                project
                    .root
                    .canonical_path
                    .file_name()
                    .and_then(|value| value.to_str())
            })
            .unwrap_or("project");
        let normalized_name = normalize_name(base_name);
        if normalized_name.is_empty() {
            return Err(DesiredStateError::new(
                "project name must contain an ASCII letter or digit",
            ));
        }
        let name = ContainerName {
            value: format!("{}-{}", normalized_name, path_hash(&project.root)),
        };
        let image = image_ref()?;
        let mounts = MountSet {
            values: vec![
                Mount {
                    host_path: project.root.canonical_path.clone(),
                    container_path: "/workspace".into(),
                    read_only: false,
                },
                Mount {
                    host_path: project.state_paths.home_dir.clone(),
                    container_path: "/home/spawnbx".into(),
                    read_only: false,
                },
                Mount {
                    host_path: project.state_paths.nix_dir.clone(),
                    container_path: "/var/lib/spawnbx/nix".into(),
                    read_only: false,
                },
            ],
        };
        let packages = PackageIntent {
            attributes: config.packages.clone(),
            lock_policy: super::types::LockPolicy::RespectExisting,
        };
        let canonical = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}|{:?}",
            name.value,
            image.repository,
            image.digest,
            facts.identity.username,
            facts.identity.uid,
            facts.identity.gid,
            config.shell.executable,
            package_fingerprint(&packages),
            config.network,
            grants,
            mounts,
        );
        let hash = SpecHash {
            lowercase_hex: digest(&canonical),
        };
        let labels = ManagedLabels {
            project_identity: project.root.canonical_path.display().to_string(),
            desired_hash: hash.clone(),
            image_digest: image.digest.clone(),
            schema_version: 1,
        };
        let container = ContainerSpec {
            image,
            name,
            identity: facts.identity.clone(),
            mounts,
            shell: config.shell.clone(),
            network: config.network.clone(),
            integrations: grants.clone(),
            labels,
        };

        Ok(DesiredState {
            container,
            packages,
            integrations: grants.clone(),
            hash,
        })
    }
}

fn image_ref() -> Result<ImageRef, DesiredStateError> {
    let reference = std::env::var("SPAWNBX_IMAGE").unwrap_or_else(|_| "spawnbx:dev".to_owned());
    let (repository, digest) = reference
        .split_once('@')
        .map(|(repository, digest)| (repository.to_owned(), digest.to_owned()))
        .unwrap_or_else(|| (reference, String::new()));
    if repository.is_empty() {
        return Err(DesiredStateError::new("SPAWNBX_IMAGE cannot be empty"));
    }
    Ok(ImageRef {
        repository,
        digest,
        platform: super::types::Platform {
            os: "linux".to_owned(),
            architecture: "amd64".to_owned(),
        },
    })
}

fn normalize_name(value: &str) -> String {
    let normalized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    normalized.trim_matches('-').to_owned()
}

fn path_hash(project: &ProjectRoot) -> String {
    digest(&project.canonical_path.display().to_string())[..12].to_owned()
}

fn digest(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn package_fingerprint(packages: &PackageIntent) -> String {
    packages
        .attributes
        .values
        .iter()
        .map(|package| package.attribute_path.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

#[derive(Clone, Debug)]
pub(crate) struct DesiredStateError {
    pub(crate) message: String,
}

impl DesiredStateError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
