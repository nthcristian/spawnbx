pub(crate) mod docker;
pub(crate) mod host;
pub(crate) mod nix;
pub(crate) mod process;
pub(crate) mod terminal;
pub(crate) mod workspace;

pub(crate) use docker::{DockerCliAdapter, FakeWorkloadAdapter};
pub(crate) use host::{FakeHostAdapter, LinuxHostAdapter};
pub(crate) use nix::{FakePackageAdapter, NixCliAdapter};
pub(crate) use process::{RecordingProcessAdapter, SystemProcessAdapter};
pub(crate) use terminal::{FakeTerminalAdapter, OsTerminalAdapter};
pub(crate) use workspace::{FakeWorkspaceAdapter, OsWorkspaceAdapter};
