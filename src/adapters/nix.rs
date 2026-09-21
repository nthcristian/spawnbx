use crate::reconcile::{
    AdapterError, NixReport, PackageAdapter, PackagePlan, ProcessCommand, WorkloadAdapter,
    WorkloadHandle,
};

pub(crate) struct NixCliAdapter;

pub(crate) struct FakePackageAdapter;

impl NixCliAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl FakePackageAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl PackageAdapter for NixCliAdapter {
    fn reconcile(
        &mut self,
        plan: &PackagePlan,
        target: &WorkloadHandle,
        executor: &mut dyn WorkloadAdapter,
    ) -> Result<NixReport, AdapterError> {
        let lock_existed = plan.lock_path.exists();
        if plan.lock_policy.is_advance() {
            require_success(
                executor.exec(
                    target,
                    ProcessCommand {
                        executable: "nix".into(),
                        arguments: vec![
                            "flake".into(),
                            "update".into(),
                            "--flake".into(),
                            plan.container_flake_dir.clone().into(),
                        ],
                    },
                )?,
                "nix flake update",
            )?;
        } else if !plan.lock_path.exists() {
            require_success(
                executor.exec(
                    target,
                    ProcessCommand {
                        executable: "nix".into(),
                        arguments: vec![
                            "flake".into(),
                            "lock".into(),
                            "--flake".into(),
                            plan.container_flake_dir.clone().into(),
                        ],
                    },
                )?,
                "nix flake lock",
            )?;
        }

        require_success(
            executor.exec(
                target,
                ProcessCommand {
                    executable: "nix".into(),
                    arguments: vec![
                        "build".into(),
                        "--no-update-lock-file".into(),
                        "--profile".into(),
                        plan.container_profile_path.clone().into(),
                        format!("{}#packages.x86_64-linux.default", plan.container_flake_dir)
                            .into(),
                    ],
                },
            )?,
            "nix build",
        )?;

        Ok(NixReport {
            lock_changed: plan.lock_policy.is_advance() || !lock_existed,
            profile_changed: true,
            warnings: Vec::new(),
        })
    }
}

impl PackageAdapter for FakePackageAdapter {
    fn reconcile(
        &mut self,
        _plan: &PackagePlan,
        _target: &WorkloadHandle,
        _executor: &mut dyn WorkloadAdapter,
    ) -> Result<NixReport, AdapterError> {
        Ok(NixReport {
            lock_changed: false,
            profile_changed: true,
            warnings: Vec::new(),
        })
    }
}

fn require_success(
    output: crate::reconcile::ProcessOutput,
    operation: &str,
) -> Result<(), AdapterError> {
    if output.status.success() {
        Ok(())
    } else {
        Err(AdapterError {
            category: "nix-command".to_owned(),
            message: format!("{operation}: {}", output.stderr.trim()),
        })
    }
}
