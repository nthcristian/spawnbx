#[derive(Debug, Clone)]
pub struct CommandInvocation {
    pub operation: CommandOperation,
    pub additional_package_names: Vec<String>,
    pub requested_shell: Option<String>,
    pub pipewire: bool,
    pub wayland: bool,
    pub gpu: bool,
    pub save_settings: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandOperation {
    Attach,
    Update { package_names: Vec<String> },
    Remove,
    Stop,
    Recreate,
}

#[derive(Debug, Clone)]
pub struct EnvironmentState {
    pub container_name_prefix: String,
    pub workspace_root: String,
    pub package_names: Vec<String>,
    pub pipewire: bool,
    pub wayland: bool,
    pub gpu: bool,
    pub shell_program: String,
    pub host_username: String,
    pub host_uid: u32,
    pub host_gid: u32,
}

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

pub type CommandExecutor<'a> = Box<dyn Fn(&[&str]) -> Result<CommandOutput, anyhow::Error> + 'a>;
