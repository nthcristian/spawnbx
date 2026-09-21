# spawnbx Architecture View

This is the implementation map for the approved design. It describes the
planned Rust crate, modules, structs, enums, traits, properties, and methods.
It is a design inventory, not an assertion that these types already exist.

The current repository has one binary crate. The scaffold package is named
`spawnbx_reroll`; the intended product and binary name is `spawnbx`. The
design does not currently justify splitting the project into multiple crates.

## Crate And Module Graph

```mermaid
graph TD
    crate["spawnbx binary crate"]

    crate --> main["main / composition root"]
    crate --> cli["cli"]
    crate --> reconcile["reconcile"]
    crate --> adapters["adapters"]

    subgraph reconcile_modules["reconcile modules"]
        invocation["invocation"]
        workflow["workflow / private phase machine"]
        project["project"]
        desired["desired"]
        preflight["preflight"]
        lifecycle["lifecycle"]
        packages["packages"]
        diagnostics["diagnostics"]
        ports["ports / Adapter Interfaces"]
    end

    reconcile --> invocation
    reconcile --> workflow
    reconcile --> project
    reconcile --> desired
    reconcile --> preflight
    reconcile --> lifecycle
    reconcile --> packages
    reconcile --> diagnostics
    reconcile --> ports

    subgraph adapter_modules["production Adapter modules"]
        workspace_adapter["workspace Adapter"]
        host_adapter["Linux host Adapter"]
        docker_adapter["Docker CLI Adapter"]
        nix_adapter["Nix CLI Adapter"]
        terminal_adapter["terminal Adapter"]
        process_adapter["process Adapter"]
    end

    adapters --> workspace_adapter
    adapters --> host_adapter
    adapters --> docker_adapter
    adapters --> nix_adapter
    adapters --> terminal_adapter
    adapters --> process_adapter

    project --> workspace_adapter
    preflight --> host_adapter
    lifecycle --> docker_adapter
    packages --> nix_adapter
    reconcile --> terminal_adapter
    docker_adapter --> process_adapter
    nix_adapter --> process_adapter

    classDef seam fill:#fff2cc,stroke:#b8860b,stroke-width:2px;
    class reconcile,ports seam;
```

## Caller-Facing Interface

`reconcile::run` is the only orchestration Interface exposed to the command
layer. The command layer builds an `Invocation`, supplies `Adapters`, and
renders the returned `Outcome` or `ReconcileError`.

```mermaid
classDiagram
    class Invocation {
        +PathBuf cwd
        +Operation operation
        +Overrides overrides
        +bool save
        +bool allow_missing_integrations
    }

    class Operation {
        <<enumeration>>
        Attach
        Update(package: Option~PackageName~)
        Stop
        Remove
        Doctor
    }

    class Overrides {
        +Option~ShellName~ shell
        +PackageList packages
        +Option~bool~ x11
        +Option~bool~ wayland
        +Option~bool~ pipewire
        +Option~bool~ gpu
        +Option~NetworkMode~ network
    }

    class Adapters {
        +WorkspaceAdapter workspace
        +HostAdapter host
        +WorkloadAdapter workload
        +PackageAdapter packages
        +TerminalAdapter terminal
    }

    class ReconcileModule {
        <<module>>
        +run(Invocation, Adapters) ReconcileResult
    }

    class Outcome {
        +ProjectRoot project_root
        +ContainerAction container_action
        +Vec~Diagnostic~ warnings
        +AttachmentOutcome attachment
    }

    class ContainerAction {
        <<enumeration>>
        Created
        ReusedRunning
        Restarted
        Recreated
        Stopped
        Removed
        Checked
    }

    class AttachmentOutcome {
        <<enumeration>>
        Interactive(status)
        Skipped
    }

    class ReconcileError {
        <<enumeration>>
        Configuration
        UnsafeProjectState
        Preflight
        Image
        Container
        Nix
        Attach
    }

    class ReconcileResult {
        +Outcome outcome
        +ReconcileError error
    }

    class PackageList {
        +Vec~PackageName~ values
    }

    class OptionalPath {
        +PathBuf value
    }

    class OptionalProjectConfig {
        +ProjectConfig value
    }

    Invocation --> Operation
    Invocation --> Overrides
    ReconcileModule --> Invocation
    ReconcileModule --> Adapters
    ReconcileModule --> Outcome
    ReconcileModule --> ReconcileError
    Outcome --> ContainerAction
    Outcome --> AttachmentOutcome
```

## Domain Structs And Pure Modules

These modules contain policy and pure decisions. They do not expose Docker or
Nix command syntax to callers.

```mermaid
classDiagram
    class ProjectModule {
        <<module>>
        +resolve(start: PathBuf, WorkspaceAdapter) Result~ProjectContext~
        +merge(ProjectConfig, Overrides) Result~EffectiveConfig~
        +save(ProjectRoot, EffectiveConfig, WorkspaceAdapter) Result
    }

    class ProjectContext {
        +ProjectRoot root
        +OptionalPath config_path
        +EffectiveConfig config
        +StatePaths state_paths
    }

    class ProjectConfig {
        +Option~String~ name
        +Option~ShellName~ shell
        +PackageList packages
        +IntegrationRequests integrations
        +Option~NetworkMode~ network
    }

    class IntegrationRequests {
        +Option~bool~ x11
        +Option~bool~ wayland
        +Option~bool~ pipewire
        +Option~bool~ gpu
    }

    class EffectiveConfig {
        +Option~String~ name
        +ShellName shell
        +Vec~PackageName~ packages
        +IntegrationRequests integrations
        +NetworkMode network
    }

    class StatePaths {
        +PathBuf state_dir
        +PathBuf home_dir
        +PathBuf nix_dir
        +PathBuf lock_path
    }

    class DesiredModule {
        <<module>>
        +build(ProjectContext, HostFacts, CapabilityGrants) Result~DesiredState~
    }

    class DesiredState {
        +ContainerSpec container
        +PackageIntent packages
        +CapabilityGrants integrations
        +SpecHash hash
    }

    class ContainerSpec {
        +ImageRef image
        +ContainerName name
        +HostIdentity identity
        +MountSet mounts
        +ShellName shell
        +NetworkMode network
        +CapabilityGrants integrations
        +ManagedLabels labels
    }

    class PreflightModule {
        <<module>>
        +evaluate(EffectiveConfig, HostFacts, PreflightPolicy) Result~PreflightResult~
    }

    class PreflightResult {
        +HostFacts facts
        +CapabilityGrants effective_grants
        +Vec~Diagnostic~ warnings
    }

    class PreflightPolicy {
        +bool allow_missing_integrations
        +bool prefer_rootless
    }

    class LifecycleModule {
        <<module>>
        +decide(DesiredState, Observation) Result~Transition~
    }

    class Observation {
        +ContainerName name
        +bool managed
        +bool running
        +SpecHash observed_hash
        +ImageRef observed_image
        +ManagedLabels labels
    }

    class Transition {
        <<enumeration>>
        Create
        Reuse
        Start
        Recreate
        Collision
        Noop
    }

    class PackagesModule {
        <<module>>
        +plan(PackageIntent, LockPolicy, StatePaths) Result~PackagePlan~
    }

    class PackageIntent {
        +Vec~PackageName~ attributes
        +LockPolicy lock_policy
    }

    class PackagePlan {
        +PathBuf generated_metadata
        +PathBuf lock_path
        +PathBuf profile_path
        +LockPolicy lock_policy
        +Option~PackageName~ focus
    }

    class LockPolicy {
        <<enumeration>>
        RespectExisting
        Advance
    }

    ProjectModule --> ProjectContext
    ProjectModule --> ProjectConfig
    ProjectContext --> EffectiveConfig
    ProjectContext --> StatePaths
    DesiredModule --> DesiredState
    DesiredState --> ContainerSpec
    DesiredState --> PackageIntent
    DesiredState --> PreflightResult
    PreflightModule --> PreflightResult
    PreflightModule --> PreflightPolicy
    PreflightResult --> HostFacts
    PreflightResult --> CapabilityGrants
    LifecycleModule --> Observation
    LifecycleModule --> Transition
    PackagesModule --> PackagePlan
    PackagePlan --> PackageIntent
    PackagePlan --> LockPolicy
```

## Value Types And Host Capabilities

Value types prevent raw strings and paths from leaking across the internal
Seams. Their implementations validate invariants at construction time.

```mermaid
classDiagram
    class ProjectRoot {
        +PathBuf canonical_path
    }

    class ContainerName {
        +String value
    }

    class ImageRef {
        +String repository
        +String digest
        +Platform platform
    }

    class HostIdentity {
        +u32 uid
        +u32 gid
        +Vec~u32~ supplementary_groups
    }

    class ShellName {
        +String executable
    }

    class PackageName {
        +String attribute_path
    }

    class SpecHash {
        +String lowercase_hex
    }

    class HostFacts {
        +Platform platform
        +DockerMode docker_mode
        +HostIdentity identity
        +EnvironmentFacts environment
        +SocketFacts sockets
        +DeviceFacts devices
    }

    class CapabilityRequest {
        +IntegrationId id
        +bool requested
    }

    class CapabilityGrant {
        +IntegrationId id
        +Vec~Mount~ mounts
        +Vec~EnvironmentValue~ environment
        +Vec~DeviceGrant~ devices
        +Vec~u32~ groups
    }

    class IntegrationId {
        <<enumeration>>
        X11
        Wayland
        PipeWire
        Gpu
    }

    class CapabilityGrants {
        +Vec~CapabilityGrant~ values
    }

    class NetworkMode {
        <<enumeration>>
        Bridge
        None
    }

    class DockerMode {
        <<enumeration>>
        Rootless
        Rootful
    }

    class Platform {
        +String os
        +String architecture
    }

    class MountSet {
        +Vec~Mount~ values
    }

    class Mount {
        +PathBuf host_path
        +PathBuf container_path
        +bool read_only
    }

    class ManagedLabels {
        +String project_identity
        +SpecHash desired_hash
        +String image_digest
        +u32 schema_version
    }

    class DeviceGrant {
        +PathBuf host_path
        +PathBuf container_path
    }

    class EnvironmentValue {
        +String name
        +String value
    }

    class EnvironmentFacts {
        +String display
        +String wayland_display
        +String pipewire_remote
    }

    class SocketFacts {
        +Vec~PathBuf~ paths
    }

    class DeviceFacts {
        +Vec~PathBuf~ render_nodes
        +bool nvidia_toolkit
    }

    DesiredState --> ProjectRoot
    ContainerSpec --> ContainerName
    ContainerSpec --> ImageRef
    ContainerSpec --> HostIdentity
    ContainerSpec --> ShellName
    PackageIntent --> PackageName
    DesiredState --> SpecHash
    HostFacts --> HostIdentity
    CapabilityRequest --> IntegrationId
    CapabilityGrant --> IntegrationId
    CapabilityGrants --> CapabilityGrant
    ContainerSpec --> MountSet
    ContainerSpec --> ManagedLabels
    CapabilityGrant --> Mount
    CapabilityGrant --> EnvironmentValue
    CapabilityGrant --> DeviceGrant
    HostFacts --> EnvironmentFacts
    HostFacts --> SocketFacts
    HostFacts --> DeviceFacts
```

## Adapter Interfaces

Each Adapter has a production implementation and a fake or recording
implementation. The Adapter Interface is the test Seam; callers do not need
to know whether the implementation uses a real filesystem, Docker CLI, Nix
CLI, or terminal.

```mermaid
classDiagram
    class WorkspaceAdapter {
        <<trait>>
        +discover(start: PathBuf) Result~ProjectRoot~
        +read_config(root: ProjectRoot) OptionalProjectConfig
        +write_config(root: ProjectRoot, config: ProjectConfig) Result
        +ensure_state(root: ProjectRoot) Result~StatePaths~
        +write_generated(path: PathBuf, contents: String) Result
    }

    class HostAdapter {
        <<trait>>
        +facts() Result~HostFacts~
    }

    class WorkloadAdapter {
        <<trait>>
        +ensure_image(image: ImageRef) Result
        +observe(name: ContainerName) Result~Observation~
        +apply(transition: Transition, desired: DesiredState) Result~WorkloadHandle~
        +exec(target: WorkloadHandle, argv: Vec~String~) Result~ProcessOutput~
    }

    class PackageAdapter {
        <<trait>>
        +reconcile(plan: PackagePlan, target: WorkloadHandle) Result~NixReport~
    }

    class TerminalAdapter {
        <<trait>>
        +attach(target: WorkloadHandle, shell: ShellName) Result~AttachmentOutcome~
    }

    class ProcessAdapter {
        <<trait>>
        +run(command: ProcessCommand) Result~ProcessOutput~
        +attach(command: ProcessCommand) Result~ExitStatus~
    }

    class WorkloadHandle {
        +String opaque_id
    }

    class ProcessCommand {
        +String executable
        +Vec~String~ arguments
    }

    class ProcessOutput {
        +ExitStatus status
        +String stdout
        +String stderr
    }

    class NixReport {
        +bool lock_changed
        +bool profile_changed
        +Vec~Diagnostic~ warnings
    }

    class Diagnostic {
        +String category
        +String message
        +String remediation
    }

    class OsWorkspaceAdapter {
        <<adapter>>
    }

    class LinuxHostAdapter {
        <<adapter>>
    }

    class DockerCliAdapter {
        <<adapter>>
    }

    class NixCliAdapter {
        <<adapter>>
    }

    class OsTerminalAdapter {
        <<adapter>>
    }

    class SystemProcessAdapter {
        <<adapter>>
    }

    WorkspaceAdapter <|.. OsWorkspaceAdapter
    HostAdapter <|.. LinuxHostAdapter
    WorkloadAdapter <|.. DockerCliAdapter
    PackageAdapter <|.. NixCliAdapter
    TerminalAdapter <|.. OsTerminalAdapter
    ProcessAdapter <|.. SystemProcessAdapter

    DockerCliAdapter --> ProcessAdapter
    NixCliAdapter --> ProcessAdapter
    WorkloadAdapter --> WorkloadHandle
    PackageAdapter --> PackagePlan
    PackageAdapter --> NixReport
    ProcessAdapter --> ProcessCommand
    ProcessAdapter --> ProcessOutput
```

## Reconciliation Flow

```mermaid
flowchart TD
    request["Invocation"] --> resolve["Resolve project and configuration"]
    resolve --> merge["Merge YAML and CLI overrides"]
    merge --> save{"Save requested?"}
    save -- yes --> save_config["Atomically save merged configuration"]
    save -- no --> facts["Collect host facts"]
    save_config --> facts
    facts --> preflight["Run aggregate preflight"]
    preflight --> valid{"Preflight passes?"}
    valid -- no --> diagnostics["Return diagnostics; no workload mutation"]
    valid -- yes --> operation{"Operation"}
    operation -- Doctor --> report["Return non-mutating readiness report"]
    operation -- Stop --> stop_observe["Observe managed workload"]
    operation -- Remove --> remove_observe["Observe managed workload"]
    stop_observe --> stop["Stop managed workload"]
    remove_observe --> remove["Remove managed workload; preserve state"]
    operation -- Attach --> desired["Build effective DesiredState and hash"]
    operation -- Update --> desired
    desired --> image["Ensure pinned image"]
    image --> observe["Observe named workload"]
    observe --> transition["Decide lifecycle Transition"]
    transition --> apply["Apply create, start, reuse, or recreate"]
    apply --> packages["Reconcile Nix profile and lock policy"]
    packages --> package_ok{"Packages pass?"}
    package_ok -- no --> retain["Return error; retain workload for diagnosis"]
    package_ok -- yes --> attach_check{"Attach requested?"}
    attach_check -- yes --> attach["Attach selected shell"]
    attach_check -- no --> outcome["Return Outcome"]
    attach --> outcome["Return Outcome"]
    stop --> outcome
    remove --> outcome
    report --> outcome
```

## Design Rules

- `reconcile::run` is the only caller-facing orchestration Interface.
- Pure policy modules decide; Adapters perform host and executable effects.
- The lifecycle module decides transitions; the Docker Adapter owns Docker
  flags and command ordering for a selected transition.
- The package module decides lock/profile policy; the Nix Adapter owns Nix
  commands and output parsing.
- Preflight completes before workload creation, removal, replacement, or start.
- Effective grants, not raw requests, participate in desired-state hashing.
- Tests replace Adapters at their Seams and assert outcomes and observable
  effects rather than private phase state.
- New integrations extend capability policy and realization without creating a
  second orchestration path.
