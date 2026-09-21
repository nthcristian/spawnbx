use std::ffi::OsString;

use crate::reconcile::{
    AdapterError, ContainerName, DesiredState, DeviceGrant, ImageRef, Mount, Observation,
    ProcessAdapter, ProcessCommand, ProcessOutput, Transition, WorkloadAdapter, WorkloadHandle,
};

pub(crate) struct DockerCliAdapter {
    process: Box<dyn ProcessAdapter>,
}

pub(crate) struct FakeWorkloadAdapter {
    pub(crate) observations: Vec<Observation>,
}

impl DockerCliAdapter {
    pub(crate) fn new(process: Box<dyn ProcessAdapter>) -> Self {
        Self { process }
    }
}

impl FakeWorkloadAdapter {
    pub(crate) fn new(observations: Vec<Observation>) -> Self {
        Self { observations }
    }
}

impl WorkloadAdapter for DockerCliAdapter {
    fn ensure_image(&mut self, image: &ImageRef) -> Result<(), AdapterError> {
        let reference = image_reference(image);
        let inspect = self.run(vec![
            "image".into(),
            "inspect".into(),
            reference.clone().into(),
        ]);
        if inspect.as_ref().is_ok_and(|output| output.status.success()) {
            return Ok(());
        }
        let output = self.run(vec!["pull".into(), reference.into()])?;
        require_success(output, "docker image pull")
    }

    fn observe(&mut self, name: &ContainerName) -> Result<Option<Observation>, AdapterError> {
        let output = self.run(vec![
            "inspect".into(),
            "--format".into(),
            "{{.State.Running}}|{{index .Config.Labels \"spawnbx.managed\"}}|{{index .Config.Labels \"spawnbx.hash\"}}|{{index .Config.Labels \"spawnbx.project\"}}|{{index .Config.Labels \"spawnbx.image\"}}|{{index .Config.Labels \"spawnbx.schema\"}}|{{.Config.Image}}".into(),
            name.value.clone().into(),
        ]);
        let output = match output {
            Ok(output) if output.status.success() => output,
            Ok(output)
                if {
                    let stderr = output.stderr.to_ascii_lowercase();
                    stderr.contains("no such object") || stderr.contains("no such container")
                } =>
            {
                return Ok(None);
            }
            Ok(output) => {
                return Err(AdapterError {
                    category: "docker-inspect".to_owned(),
                    message: output.stderr,
                });
            }
            Err(error) => return Err(error),
        };
        let values = output.stdout.trim().split('|').collect::<Vec<_>>();
        if values.len() != 7 {
            return Err(AdapterError {
                category: "docker-inspect".to_owned(),
                message: output.stdout,
            });
        }
        let managed = values[1] == "true";
        let observed_hash = super::super::reconcile::SpecHash {
            lowercase_hex: values[2].to_owned(),
        };
        let observed_image = ImageRef {
            repository: values[6].to_owned(),
            digest: values[6]
                .split_once('@')
                .map_or_else(String::new, |(_, digest)| digest.to_owned()),
            platform: super::super::reconcile::Platform {
                os: "linux".to_owned(),
                architecture: "amd64".to_owned(),
            },
        };
        Ok(Some(Observation {
            name: name.clone(),
            managed,
            running: values[0] == "true",
            observed_hash,
            observed_image: observed_image.clone(),
            labels: super::super::reconcile::ManagedLabels {
                project_identity: values[3].to_owned(),
                desired_hash: super::super::reconcile::SpecHash {
                    lowercase_hex: values[2].to_owned(),
                },
                image_digest: values[4].to_owned(),
                schema_version: values[5].parse().unwrap_or_default(),
            },
        }))
    }

    fn apply(
        &mut self,
        transition: &Transition,
        desired: &DesiredState,
    ) -> Result<WorkloadHandle, AdapterError> {
        let name = &desired.container.name.value;
        match transition {
            Transition::Create => {
                self.create(desired)?;
                self.start(name)?;
            }
            Transition::Recreate => {
                require_success(
                    self.run(vec!["rm".into(), "-f".into(), name.clone().into()])?,
                    "docker remove before recreate",
                )?;
                self.create(desired)?;
                self.start(name)?;
            }
            Transition::Start => self.start(name)?,
            Transition::Reuse | Transition::Noop => {}
            Transition::Stop => {
                require_success(
                    self.run(vec!["stop".into(), name.clone().into()])?,
                    "docker stop",
                )?;
            }
            Transition::Remove => {
                require_success(
                    self.run(vec!["rm".into(), "-f".into(), name.clone().into()])?,
                    "docker remove",
                )?;
            }
            Transition::Collision => {
                return Err(AdapterError {
                    category: "name-collision".to_owned(),
                    message: format!("unmanaged workload occupies {name}"),
                });
            }
        }
        Ok(WorkloadHandle {
            opaque_id: name.clone(),
        })
    }

    fn exec(
        &mut self,
        target: &WorkloadHandle,
        command: ProcessCommand,
    ) -> Result<ProcessOutput, AdapterError> {
        let mut arguments = vec![
            "exec".into(),
            target.opaque_id.clone().into(),
            command.executable,
        ];
        arguments.extend(command.arguments);
        self.run(arguments)
    }
}

impl DockerCliAdapter {
    fn create(&mut self, desired: &DesiredState) -> Result<(), AdapterError> {
        let spec = &desired.container;
        let mut arguments = vec![
            "create".into(),
            "--name".into(),
            spec.name.value.clone().into(),
            "--workdir".into(),
            "/workspace".into(),
            "--user".into(),
            format!("{}:{}", spec.identity.uid, spec.identity.gid).into(),
            "--env".into(),
            "HOME=/home/spawnbx".into(),
            "--env".into(),
            "PATH=/home/spawnbx/.spawnbx-profile/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
            "--network".into(),
            network_name(&spec.network).into(),
            "--label".into(),
            "spawnbx.managed=true".into(),
            "--label".into(),
            format!("spawnbx.project={}", spec.labels.project_identity).into(),
            "--label".into(),
            format!("spawnbx.hash={}", spec.labels.desired_hash.lowercase_hex).into(),
            "--label".into(),
            format!("spawnbx.image={}", spec.image.digest).into(),
            "--label".into(),
            "spawnbx.schema=1".into(),
        ];
        for mount in &spec.mounts.values {
            arguments.extend(mount_arguments(mount));
        }
        for grant in &spec.integrations.values {
            for mount in &grant.mounts {
                arguments.extend(mount_arguments(mount));
            }
            for environment in &grant.environment {
                arguments.extend([
                    "--env".into(),
                    format!("{}={}", environment.name, environment.value).into(),
                ]);
            }
            for device in &grant.devices {
                arguments.extend(device_arguments(device));
            }
            for group in &grant.groups {
                arguments.extend(["--group-add".into(), group.to_string().into()]);
            }
            if matches!(grant.id, super::super::reconcile::IntegrationId::Gpu)
                && grant.devices.is_empty()
            {
                arguments.extend(["--gpus".into(), "all".into()]);
            }
        }
        arguments.extend([
            image_reference(&spec.image).into(),
            "sleep".into(),
            "infinity".into(),
        ]);
        require_success(self.run(arguments)?, "docker create")
    }

    fn start(&mut self, name: &str) -> Result<(), AdapterError> {
        require_success(
            self.run(vec!["start".into(), name.to_owned().into()])?,
            "docker start",
        )
    }

    fn run(&mut self, arguments: Vec<OsString>) -> Result<ProcessOutput, AdapterError> {
        self.process.run(ProcessCommand {
            executable: "docker".into(),
            arguments,
        })
    }
}

impl WorkloadAdapter for FakeWorkloadAdapter {
    fn ensure_image(&mut self, _image: &ImageRef) -> Result<(), AdapterError> {
        Ok(())
    }

    fn observe(&mut self, name: &ContainerName) -> Result<Option<Observation>, AdapterError> {
        Ok(self
            .observations
            .iter()
            .find(|observation| observation.name.value == name.value)
            .cloned())
    }

    fn apply(
        &mut self,
        _transition: &Transition,
        desired: &DesiredState,
    ) -> Result<WorkloadHandle, AdapterError> {
        Ok(WorkloadHandle {
            opaque_id: desired.container.name.value.clone(),
        })
    }

    fn exec(
        &mut self,
        _target: &WorkloadHandle,
        _command: ProcessCommand,
    ) -> Result<ProcessOutput, AdapterError> {
        Err(AdapterError {
            category: "fake-exec".to_owned(),
            message: "fake workload execution is not configured".to_owned(),
        })
    }
}

fn image_reference(image: &ImageRef) -> String {
    if image.digest.is_empty() {
        image.repository.clone()
    } else {
        format!("{}@{}", image.repository, image.digest)
    }
}

fn network_name(network: &super::super::reconcile::NetworkMode) -> &'static str {
    match network {
        super::super::reconcile::NetworkMode::Bridge => "bridge",
        super::super::reconcile::NetworkMode::None => "none",
    }
}

fn mount_arguments(mount: &Mount) -> Vec<OsString> {
    let mut value = format!(
        "type=bind,src={},dst={}",
        mount.host_path.display(),
        mount.container_path.display()
    );
    if mount.read_only {
        value.push_str(",readonly");
    }
    vec!["--mount".into(), value.into()]
}

fn device_arguments(device: &DeviceGrant) -> Vec<OsString> {
    vec![
        "--device".into(),
        format!(
            "{}:{}",
            device.host_path.display(),
            device.container_path.display()
        )
        .into(),
    ]
}

fn require_success(output: ProcessOutput, operation: &str) -> Result<(), AdapterError> {
    if output.status.success() {
        return Ok(());
    }
    Err(AdapterError {
        category: "docker-command".to_owned(),
        message: format!("{operation}: {}", output.stderr.trim()),
    })
}
