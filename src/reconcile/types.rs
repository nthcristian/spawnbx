use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitStatus;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub(crate) struct Invocation {
    pub(crate) cwd: PathBuf,
    pub(crate) operation: Operation,
    pub(crate) overrides: Overrides,
    pub(crate) save: bool,
    pub(crate) allow_missing_integrations: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum Operation {
    Attach,
    Update { package: Option<PackageName> },
    Stop,
    Remove,
    Doctor,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Overrides {
    pub(crate) shell: Option<String>,
    pub(crate) packages: Option<Vec<String>>,
    pub(crate) x11: Option<bool>,
    pub(crate) wayland: Option<bool>,
    pub(crate) pipewire: Option<bool>,
    pub(crate) gpu: Option<bool>,
    pub(crate) network: Option<NetworkMode>,
}

#[derive(Clone, Debug)]
pub(crate) struct Outcome {
    pub(crate) project_root: ProjectRoot,
    pub(crate) container_action: ContainerAction,
    pub(crate) warnings: Vec<Diagnostic>,
    pub(crate) attachment: AttachmentOutcome,
}

#[derive(Clone, Debug)]
pub(crate) enum ContainerAction {
    Created,
    ReusedRunning,
    Restarted,
    Recreated,
    Stopped,
    Removed,
    Checked,
}

#[derive(Clone, Debug)]
pub(crate) enum AttachmentOutcome {
    Interactive(ExitStatus),
    Skipped,
}

#[derive(Clone, Debug)]
pub(crate) enum ReconcileError {
    Configuration(Diagnostic),
    UnsafeProjectState(Diagnostic),
    Preflight(Vec<Diagnostic>),
    Image(Diagnostic),
    Container(Diagnostic),
    Nix(Diagnostic),
    Attach(Diagnostic),
}

pub(crate) type ReconcileResult = Result<Outcome, ReconcileError>;

#[derive(Clone, Debug)]
pub(crate) struct ProjectContext {
    pub(crate) root: ProjectRoot,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) config: ProjectConfig,
    pub(crate) state_paths: StatePaths,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectConfig {
    pub(crate) name: Option<String>,
    pub(crate) shell: Option<String>,
    pub(crate) packages: Option<Vec<String>>,
    pub(crate) x11: Option<bool>,
    pub(crate) wayland: Option<bool>,
    pub(crate) pipewire: Option<bool>,
    pub(crate) gpu: Option<bool>,
    pub(crate) network: Option<NetworkMode>,
}

#[derive(Clone, Debug)]
pub(crate) struct EffectiveConfig {
    pub(crate) name: Option<String>,
    pub(crate) shell: ShellName,
    pub(crate) packages: PackageList,
    pub(crate) integrations: IntegrationRequests,
    pub(crate) network: NetworkMode,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct IntegrationRequests {
    pub(crate) x11: bool,
    pub(crate) wayland: bool,
    pub(crate) pipewire: bool,
    pub(crate) gpu: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct StatePaths {
    pub(crate) state_dir: PathBuf,
    pub(crate) home_dir: PathBuf,
    pub(crate) nix_dir: PathBuf,
    pub(crate) lock_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct DesiredState {
    pub(crate) container: ContainerSpec,
    pub(crate) packages: PackageIntent,
    pub(crate) integrations: CapabilityGrants,
    pub(crate) hash: SpecHash,
}

#[derive(Clone, Debug)]
pub(crate) struct ContainerSpec {
    pub(crate) image: ImageRef,
    pub(crate) name: ContainerName,
    pub(crate) identity: HostIdentity,
    pub(crate) mounts: MountSet,
    pub(crate) shell: ShellName,
    pub(crate) network: NetworkMode,
    pub(crate) integrations: CapabilityGrants,
    pub(crate) labels: ManagedLabels,
}

#[derive(Clone, Debug)]
pub(crate) struct PreflightResult {
    pub(crate) facts: HostFacts,
    pub(crate) effective_grants: CapabilityGrants,
    pub(crate) warnings: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub(crate) struct PreflightPolicy {
    pub(crate) allow_missing_integrations: bool,
    pub(crate) prefer_rootless: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Observation {
    pub(crate) name: ContainerName,
    pub(crate) managed: bool,
    pub(crate) running: bool,
    pub(crate) observed_hash: SpecHash,
    pub(crate) observed_image: ImageRef,
    pub(crate) labels: ManagedLabels,
}

#[derive(Clone, Debug)]
pub(crate) enum Transition {
    Create,
    Reuse,
    Start,
    Recreate,
    Stop,
    Remove,
    Collision,
    Noop,
}

#[derive(Clone, Debug)]
pub(crate) struct PackageIntent {
    pub(crate) attributes: PackageList,
    pub(crate) lock_policy: LockPolicy,
}

#[derive(Clone, Debug)]
pub(crate) struct PackagePlan {
    pub(crate) generated_metadata: PathBuf,
    pub(crate) lock_path: PathBuf,
    pub(crate) profile_path: PathBuf,
    pub(crate) container_flake_dir: String,
    pub(crate) container_profile_path: String,
    pub(crate) flake_contents: String,
    pub(crate) lock_policy: LockPolicy,
    pub(crate) focus: Option<PackageName>,
}

#[derive(Clone, Debug)]
pub(crate) enum LockPolicy {
    RespectExisting,
    Advance,
}

impl LockPolicy {
    pub(crate) fn is_advance(&self) -> bool {
        matches!(self, Self::Advance)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PackageList {
    pub(crate) values: Vec<PackageName>,
}

#[derive(Clone, Debug)]
pub(crate) struct ProjectRoot {
    pub(crate) canonical_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct ContainerName {
    pub(crate) value: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ImageRef {
    pub(crate) repository: String,
    pub(crate) digest: String,
    pub(crate) platform: Platform,
}

#[derive(Clone, Debug)]
pub(crate) struct HostIdentity {
    pub(crate) username: String,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) supplementary_groups: Vec<u32>,
}

#[derive(Clone, Debug)]
pub(crate) struct ShellName {
    pub(crate) executable: String,
}

#[derive(Clone, Debug)]
pub(crate) struct PackageName {
    pub(crate) attribute_path: String,
}

#[derive(Clone, Debug)]
pub(crate) struct SpecHash {
    pub(crate) lowercase_hex: String,
}

#[derive(Clone, Debug)]
pub(crate) struct HostFacts {
    pub(crate) platform: Platform,
    pub(crate) docker_mode: DockerMode,
    pub(crate) identity: HostIdentity,
    pub(crate) environment: EnvironmentFacts,
    pub(crate) sockets: SocketFacts,
    pub(crate) devices: DeviceFacts,
}

#[derive(Clone, Debug)]
pub(crate) struct CapabilityRequest {
    pub(crate) id: IntegrationId,
    pub(crate) requested: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct CapabilityGrant {
    pub(crate) id: IntegrationId,
    pub(crate) mounts: Vec<Mount>,
    pub(crate) environment: Vec<EnvironmentValue>,
    pub(crate) devices: Vec<DeviceGrant>,
    pub(crate) groups: Vec<u32>,
}

#[derive(Clone, Debug)]
pub(crate) enum IntegrationId {
    X11,
    Wayland,
    PipeWire,
    Gpu,
}

#[derive(Clone, Debug)]
pub(crate) struct CapabilityGrants {
    pub(crate) values: Vec<CapabilityGrant>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum NetworkMode {
    Bridge,
    None,
}

#[derive(Clone, Debug)]
pub(crate) enum DockerMode {
    Rootless,
    Rootful,
}

#[derive(Clone, Debug)]
pub(crate) struct Platform {
    pub(crate) os: String,
    pub(crate) architecture: String,
}

#[derive(Clone, Debug)]
pub(crate) struct MountSet {
    pub(crate) values: Vec<Mount>,
}

#[derive(Clone, Debug)]
pub(crate) struct Mount {
    pub(crate) host_path: PathBuf,
    pub(crate) container_path: PathBuf,
    pub(crate) read_only: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ManagedLabels {
    pub(crate) project_identity: String,
    pub(crate) desired_hash: SpecHash,
    pub(crate) image_digest: String,
    pub(crate) schema_version: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct DeviceGrant {
    pub(crate) host_path: PathBuf,
    pub(crate) container_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct EnvironmentValue {
    pub(crate) name: String,
    pub(crate) value: String,
}

#[derive(Clone, Debug)]
pub(crate) struct EnvironmentFacts {
    pub(crate) display: Option<String>,
    pub(crate) wayland_display: Option<String>,
    pub(crate) pipewire_remote: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct SocketFacts {
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub(crate) struct DeviceFacts {
    pub(crate) render_nodes: Vec<PathBuf>,
    pub(crate) nvidia_toolkit: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Diagnostic {
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) remediation: String,
}

#[derive(Debug)]
pub(crate) struct WorkloadHandle {
    pub(crate) opaque_id: String,
    pub(crate) identity: HostIdentity,
}

#[derive(Clone, Debug)]
pub(crate) struct ProcessCommand {
    pub(crate) executable: OsString,
    pub(crate) arguments: Vec<OsString>,
}

#[derive(Debug)]
pub(crate) struct ProcessOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

#[derive(Clone, Debug)]
pub(crate) struct NixReport {
    pub(crate) lock_changed: bool,
    pub(crate) profile_changed: bool,
    pub(crate) warnings: Vec<Diagnostic>,
}
