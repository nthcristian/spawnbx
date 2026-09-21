use std::path::PathBuf;

use super::types::{
    AttachmentOutcome, ContainerName, DesiredState, HostFacts, HostIdentity, ImageRef, NixReport,
    Observation, PackagePlan, ProcessCommand, ProcessOutput, ProjectConfig, ProjectRoot, ShellName,
    StatePaths, Transition, WorkloadHandle,
};

pub(crate) struct Adapters<'a> {
    pub(crate) workspace: &'a mut dyn WorkspaceAdapter,
    pub(crate) host: &'a mut dyn HostAdapter,
    pub(crate) workload: &'a mut dyn WorkloadAdapter,
    pub(crate) packages: &'a mut dyn PackageAdapter,
    pub(crate) terminal: &'a mut dyn TerminalAdapter,
}

pub(crate) trait WorkspaceAdapter {
    fn discover(&mut self, start: PathBuf) -> Result<ProjectRoot, AdapterError>;

    fn read_config(&mut self, root: &ProjectRoot) -> Result<Option<ProjectConfig>, AdapterError>;

    fn write_config(
        &mut self,
        root: &ProjectRoot,
        config: &ProjectConfig,
    ) -> Result<(), AdapterError>;

    fn ensure_state(
        &mut self,
        root: &ProjectRoot,
        create: bool,
    ) -> Result<StatePaths, AdapterError>;

    fn write_generated(&mut self, path: PathBuf, contents: String) -> Result<(), AdapterError>;
}

pub(crate) trait HostAdapter {
    fn facts(&mut self) -> Result<HostFacts, AdapterError>;
}

pub(crate) trait WorkloadAdapter {
    fn ensure_image(&mut self, image: &ImageRef) -> Result<(), AdapterError>;

    fn observe(&mut self, name: &ContainerName) -> Result<Option<Observation>, AdapterError>;

    fn apply(
        &mut self,
        transition: &Transition,
        desired: &DesiredState,
    ) -> Result<WorkloadHandle, AdapterError>;

    fn validate_user(
        &mut self,
        target: &WorkloadHandle,
        identity: &HostIdentity,
    ) -> Result<(), AdapterError>;

    fn exec(
        &mut self,
        target: &WorkloadHandle,
        command: ProcessCommand,
    ) -> Result<ProcessOutput, AdapterError>;
}

pub(crate) trait PackageAdapter {
    fn reconcile(
        &mut self,
        plan: &PackagePlan,
        target: &WorkloadHandle,
        executor: &mut dyn WorkloadAdapter,
    ) -> Result<NixReport, AdapterError>;
}

pub(crate) trait TerminalAdapter {
    fn attach(
        &mut self,
        target: &WorkloadHandle,
        shell: &ShellName,
    ) -> Result<AttachmentOutcome, AdapterError>;
}

pub(crate) trait ProcessAdapter {
    fn run(&mut self, command: ProcessCommand) -> Result<ProcessOutput, AdapterError>;

    fn attach(&mut self, command: ProcessCommand)
    -> Result<std::process::ExitStatus, AdapterError>;
}

#[derive(Clone, Debug)]
pub(crate) struct AdapterError {
    pub(crate) category: String,
    pub(crate) message: String,
}
