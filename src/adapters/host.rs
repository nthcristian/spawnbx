use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::reconcile::{
    AdapterError, DeviceFacts, DockerMode, EnvironmentFacts, HostAdapter, HostFacts, HostIdentity,
    Platform, SocketFacts,
};

pub(crate) struct LinuxHostAdapter;

pub(crate) struct FakeHostAdapter {
    pub(crate) facts: HostFacts,
}

impl LinuxHostAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl FakeHostAdapter {
    pub(crate) fn new(facts: HostFacts) -> Self {
        Self { facts }
    }
}

impl HostAdapter for LinuxHostAdapter {
    fn facts(&mut self) -> Result<HostFacts, AdapterError> {
        let identity = host_identity()?;
        let runtime_dir = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
        let wayland_socket = runtime_dir.as_ref().and_then(|runtime| {
            env::var_os("WAYLAND_DISPLAY")
                .map(PathBuf::from)
                .map(|display| runtime.join(display))
        });
        let pipewire_socket = runtime_dir
            .as_ref()
            .map(|runtime| runtime.join("pipewire-0"));
        let mut socket_paths = vec![PathBuf::from("/tmp/.X11-unix")];
        if let Some(path) = wayland_socket {
            socket_paths.push(path);
        }
        if let Some(path) = pipewire_socket {
            socket_paths.push(path);
        }

        let render_nodes = fs::read_dir("/dev/dri")
            .ok()
            .into_iter()
            .flat_map(|entries| entries.flatten())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("renderD"))
            })
            .collect();

        Ok(HostFacts {
            platform: Platform {
                os: env::consts::OS.to_owned(),
                architecture: env::consts::ARCH.to_owned(),
            },
            docker_mode: docker_mode(),
            identity,
            environment: EnvironmentFacts {
                display: env::var("DISPLAY").ok(),
                wayland_display: env::var("WAYLAND_DISPLAY").ok(),
                pipewire_remote: env::var("PIPEWIRE_REMOTE").ok(),
            },
            sockets: SocketFacts {
                paths: socket_paths,
            },
            devices: DeviceFacts {
                render_nodes,
                nvidia_toolkit: command_available("nvidia-container-cli"),
            },
        })
    }
}

impl HostAdapter for FakeHostAdapter {
    fn facts(&mut self) -> Result<HostFacts, AdapterError> {
        Ok(self.facts.clone())
    }
}

fn host_identity() -> Result<HostIdentity, AdapterError> {
    let uid = id_value("-u")?;
    let gid = id_value("-g")?;
    let groups = id_output("-G")?
        .split_whitespace()
        .map(|value| {
            value.parse::<u32>().map_err(|error| AdapterError {
                category: "identity".to_owned(),
                message: error.to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(HostIdentity {
        uid,
        gid,
        supplementary_groups: groups,
    })
}

fn id_value(argument: &str) -> Result<u32, AdapterError> {
    id_output(argument)?
        .trim()
        .parse::<u32>()
        .map_err(|error| AdapterError {
            category: "identity".to_owned(),
            message: error.to_string(),
        })
}

fn id_output(argument: &str) -> Result<String, AdapterError> {
    let output = Command::new("id")
        .arg(argument)
        .output()
        .map_err(|error| AdapterError {
            category: "identity".to_owned(),
            message: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(AdapterError {
            category: "identity".to_owned(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn docker_mode() -> DockerMode {
    if env::var("SPAWNBX_DOCKER_MODE").as_deref() == Ok("rootful") {
        return DockerMode::Rootful;
    }
    if env::var("SPAWNBX_DOCKER_MODE").as_deref() == Ok("rootless") {
        return DockerMode::Rootless;
    }
    let output = Command::new("docker")
        .args(["info", "--format", "{{.SecurityOptions}}"])
        .output();
    match output {
        Ok(output) if String::from_utf8_lossy(&output.stdout).contains("rootless") => {
            DockerMode::Rootless
        }
        _ => DockerMode::Rootful,
    }
}

fn command_available(command: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {command}")])
        .output()
        .is_ok_and(|output| output.status.success())
}
