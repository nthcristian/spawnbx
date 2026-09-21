use std::env;
use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::path::PathBuf;

use super::types::{
    CapabilityGrant, CapabilityGrants, Diagnostic, EffectiveConfig, HostFacts, IntegrationId,
    Mount, PreflightPolicy, PreflightResult,
};

pub(crate) trait PreflightModule {
    fn evaluate(
        &mut self,
        config: &EffectiveConfig,
        facts: &HostFacts,
        policy: &PreflightPolicy,
    ) -> Result<PreflightResult, PreflightError>;
}

pub(crate) struct PreflightEvaluator;

impl PreflightEvaluator {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl PreflightModule for PreflightEvaluator {
    fn evaluate(
        &mut self,
        config: &EffectiveConfig,
        facts: &HostFacts,
        policy: &PreflightPolicy,
    ) -> Result<PreflightResult, PreflightError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if facts.platform.os != "linux" || facts.platform.architecture != "x86_64" {
            errors.push(diagnostic(
                "unsupported-platform",
                "spawnbx currently supports linux/amd64 hosts only",
                "run spawnbx on a linux/amd64 host",
            ));
        }
        if matches!(facts.docker_mode, super::types::DockerMode::Rootful) {
            warnings.push(diagnostic(
                "rootful-docker",
                "Docker is running in rootful mode",
                "configure rootless Docker when practical",
            ));
        }

        let mut grants = Vec::new();
        integration(
            config.integrations.x11,
            policy,
            &mut errors,
            &mut warnings,
            &mut grants,
            x11_grant(facts),
        );
        integration(
            config.integrations.wayland,
            policy,
            &mut errors,
            &mut warnings,
            &mut grants,
            wayland_grant(facts),
        );
        integration(
            config.integrations.pipewire,
            policy,
            &mut errors,
            &mut warnings,
            &mut grants,
            pipewire_grant(facts),
        );
        integration(
            config.integrations.gpu,
            policy,
            &mut errors,
            &mut warnings,
            &mut grants,
            gpu_grant(facts),
        );

        if errors.is_empty() {
            Ok(PreflightResult {
                facts: facts.clone(),
                effective_grants: CapabilityGrants { values: grants },
                warnings,
            })
        } else {
            Err(PreflightError {
                diagnostics: errors,
            })
        }
    }
}

fn integration(
    requested: bool,
    policy: &PreflightPolicy,
    errors: &mut Vec<Diagnostic>,
    warnings: &mut Vec<Diagnostic>,
    grants: &mut Vec<CapabilityGrant>,
    result: Result<CapabilityGrant, Diagnostic>,
) {
    if !requested {
        return;
    }
    match result {
        Ok(grant) => grants.push(grant),
        Err(error) if policy.allow_missing_integrations => {
            warnings.push(Diagnostic {
                category: "missing-integration-omitted".to_owned(),
                message: error.message,
                remediation: error.remediation,
            });
        }
        Err(error) => errors.push(error),
    }
}

fn x11_grant(facts: &HostFacts) -> Result<CapabilityGrant, Diagnostic> {
    let display = facts.environment.display.clone().ok_or_else(|| {
        diagnostic(
            "x11-display",
            "DISPLAY is not set",
            "set DISPLAY in the active X11 session",
        )
    })?;
    let socket = PathBuf::from("/tmp/.X11-unix");
    if !socket.is_dir() {
        return Err(diagnostic(
            "x11-socket",
            "the X11 socket directory is unavailable",
            "start an X11 session or disable X11 forwarding",
        ));
    }
    let mut mounts = vec![Mount {
        host_path: socket.clone(),
        container_path: socket,
        read_only: true,
    }];
    if let Some(authority) = env::var_os("XAUTHORITY")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".Xauthority")))
        .filter(|path| path.is_file())
    {
        mounts.push(Mount {
            host_path: authority,
            container_path: "/home/spawnbx/.Xauthority".into(),
            read_only: true,
        });
    }
    Ok(CapabilityGrant {
        id: IntegrationId::X11,
        mounts,
        environment: vec![super::types::EnvironmentValue {
            name: "DISPLAY".to_owned(),
            value: display,
        }],
        devices: Vec::new(),
        groups: Vec::new(),
    })
}

fn wayland_grant(facts: &HostFacts) -> Result<CapabilityGrant, Diagnostic> {
    let display = facts.environment.wayland_display.clone().ok_or_else(|| {
        diagnostic(
            "wayland-display",
            "WAYLAND_DISPLAY is not set",
            "set WAYLAND_DISPLAY in the active Wayland session",
        )
    })?;
    let runtime = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| {
            diagnostic(
                "wayland-runtime",
                "XDG_RUNTIME_DIR is not set",
                "run spawnbx from the active user session",
            )
        })?;
    let socket = runtime.join(&display);
    if !is_socket(&socket) {
        return Err(diagnostic(
            "wayland-socket",
            format!("Wayland socket is unavailable: {}", socket.display()),
            "keep the Wayland session active or disable Wayland forwarding",
        ));
    }
    Ok(CapabilityGrant {
        id: IntegrationId::Wayland,
        mounts: vec![Mount {
            host_path: socket,
            container_path: runtime_path(facts.identity.uid, &display),
            read_only: false,
        }],
        environment: vec![
            super::types::EnvironmentValue {
                name: "WAYLAND_DISPLAY".to_owned(),
                value: display,
            },
            super::types::EnvironmentValue {
                name: "XDG_RUNTIME_DIR".to_owned(),
                value: runtime_path(facts.identity.uid, "").display().to_string(),
            },
        ],
        devices: Vec::new(),
        groups: Vec::new(),
    })
}

fn pipewire_grant(facts: &HostFacts) -> Result<CapabilityGrant, Diagnostic> {
    let runtime = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| {
            diagnostic(
                "pipewire-runtime",
                "XDG_RUNTIME_DIR is not set",
                "run spawnbx from the active user session",
            )
        })?;
    let remote = facts
        .environment
        .pipewire_remote
        .clone()
        .unwrap_or_else(|| "pipewire-0".to_owned());
    let socket_name = PathBuf::from(&remote)
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("pipewire-0"));
    let socket = if PathBuf::from(&remote).is_absolute() {
        PathBuf::from(&remote)
    } else {
        runtime.join(&remote)
    };
    if !is_socket(&socket) {
        return Err(diagnostic(
            "pipewire-socket",
            format!("PipeWire socket is unavailable: {}", socket.display()),
            "start PipeWire or disable PipeWire forwarding",
        ));
    }
    Ok(CapabilityGrant {
        id: IntegrationId::PipeWire,
        mounts: vec![Mount {
            host_path: socket,
            container_path: runtime_path(facts.identity.uid, &socket_name.display().to_string()),
            read_only: false,
        }],
        environment: vec![super::types::EnvironmentValue {
            name: "PIPEWIRE_REMOTE".to_owned(),
            value: if PathBuf::from(&remote).is_absolute() {
                runtime_path(facts.identity.uid, &socket_name.display().to_string())
                    .display()
                    .to_string()
            } else {
                remote
            },
        }],
        devices: Vec::new(),
        groups: Vec::new(),
    })
}

fn runtime_path(uid: u32, name: &str) -> PathBuf {
    PathBuf::from(format!("/run/user/{uid}")).join(name)
}

fn gpu_grant(facts: &HostFacts) -> Result<CapabilityGrant, Diagnostic> {
    if facts.devices.nvidia_toolkit {
        return Ok(CapabilityGrant {
            id: IntegrationId::Gpu,
            mounts: Vec::new(),
            environment: Vec::new(),
            devices: Vec::new(),
            groups: Vec::new(),
        });
    }
    if facts.devices.render_nodes.is_empty() {
        return Err(diagnostic(
            "gpu-unavailable",
            "no NVIDIA toolkit or DRM render nodes were detected",
            "install the required GPU runtime or disable GPU forwarding",
        ));
    }
    let devices = facts
        .devices
        .render_nodes
        .iter()
        .map(|path| super::types::DeviceGrant {
            host_path: path.clone(),
            container_path: path.clone(),
        })
        .collect();
    Ok(CapabilityGrant {
        id: IntegrationId::Gpu,
        mounts: Vec::new(),
        environment: Vec::new(),
        devices,
        groups: render_group_ids(),
    })
}

fn render_group_ids() -> Vec<u32> {
    fs::read_to_string("/etc/group")
        .ok()
        .into_iter()
        .flat_map(|contents| contents.lines().map(str::to_owned).collect::<Vec<_>>())
        .filter_map(|line| {
            let mut fields = line.split(':');
            let name = fields.next()?;
            let _password = fields.next()?;
            let gid = fields.next()?.parse().ok()?;
            (matches!(name, "render" | "video")).then_some(gid)
        })
        .collect()
}

fn diagnostic(
    category: impl Into<String>,
    message: impl Into<String>,
    remediation: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        category: category.into(),
        message: message.into(),
        remediation: remediation.into(),
    }
}

fn is_socket(path: &PathBuf) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.file_type().is_socket())
        .unwrap_or(false)
}

#[derive(Clone, Debug)]
pub(crate) struct PreflightError {
    pub(crate) diagnostics: Vec<Diagnostic>,
}
