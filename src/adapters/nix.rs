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
                executor.exec(target, flake_command("update", &plan.container_flake_dir))?,
                "nix flake update",
            )?;
        } else if !plan.lock_path.exists() {
            require_success(
                executor.exec(target, flake_command("lock", &plan.container_flake_dir))?,
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

fn flake_command(operation: &str, directory: &str) -> ProcessCommand {
    let arguments = if operation == "update" {
        vec![
            "flake".into(),
            operation.into(),
            "--flake".into(),
            directory.into(),
        ]
    } else {
        vec!["flake".into(), operation.into(), directory.into()]
    };
    ProcessCommand {
        executable: "nix".into(),
        arguments,
    }
}

#[cfg(test)]
mod tests {
    use super::flake_command;
    use std::ffi::OsString;

    #[test]
    fn flake_lock_uses_positional_directory() {
        assert_eq!(
            flake_command("lock", "/workspace/.spawnbx/nix").arguments,
            vec![
                OsString::from("flake"),
                OsString::from("lock"),
                OsString::from("/workspace/.spawnbx/nix")
            ]
        );
    }
}
