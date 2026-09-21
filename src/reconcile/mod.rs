mod desired;
mod diagnostics;
mod lifecycle;
mod packages;
mod ports;
mod preflight;
mod project;
mod types;

use desired::{DesiredStateBuilder, DesiredStateModule};
use lifecycle::{LifecycleDecider, LifecycleModule};
use packages::{PackagePlanner, PackagesModule};
pub(crate) use ports::*;
use preflight::{PreflightEvaluator, PreflightModule};
use project::{ProjectModule, ProjectResolver};
pub(crate) use types::*;

pub(crate) fn run(invocation: Invocation, adapters: &mut Adapters<'_>) -> ReconcileResult {
    let mut project_module = ProjectResolver::new();
    let create_state = !matches!(invocation.operation, Operation::Doctor);
    let project = project_module
        .resolve(invocation.cwd.clone(), adapters.workspace, create_state)
        .map_err(|error| ReconcileError::Configuration(diagnostic(error.message)))?;
    let effective = project_module
        .merge(project.config.clone(), &invocation)
        .map_err(|error| ReconcileError::Configuration(diagnostic(error.message)))?;
    if invocation.save {
        project_module
            .save(&project.root, &effective, adapters.workspace)
            .map_err(|error| ReconcileError::Configuration(diagnostic(error.message)))?;
    }

    let facts = adapters
        .host
        .facts()
        .map_err(|error| ReconcileError::Preflight(vec![diagnostic(error.message)]))?;
    let mut preflight = PreflightEvaluator::new();
    let preflight = preflight
        .evaluate(
            &effective,
            &facts,
            &PreflightPolicy {
                allow_missing_integrations: invocation.allow_missing_integrations,
                prefer_rootless: true,
            },
        )
        .map_err(|error| ReconcileError::Preflight(error.diagnostics))?;

    let mut desired_builder = DesiredStateBuilder::new();
    let mut desired = desired_builder
        .build(&project, &effective, &facts, &preflight.effective_grants)
        .map_err(|error| ReconcileError::Configuration(diagnostic(error.message)))?;

    let update_package = match &invocation.operation {
        Operation::Update { package } => package.clone(),
        _ => None,
    };
    let is_update = matches!(invocation.operation, Operation::Update { .. });

    match invocation.operation {
        Operation::Doctor => {
            adapters
                .workload
                .ensure_image(&desired.container.image)
                .map_err(|error| ReconcileError::Image(diagnostic(error.message)))?;
            Ok(Outcome {
                project_root: project.root,
                container_action: ContainerAction::Checked,
                warnings: preflight.warnings,
                attachment: AttachmentOutcome::Skipped,
            })
        }
        Operation::Stop | Operation::Remove => {
            let observation = adapters
                .workload
                .observe(&desired.container.name)
                .map_err(|error| ReconcileError::Container(diagnostic(error.message)))?;
            if let Some(ref observation) = observation {
                if !observation.managed {
                    return Err(ReconcileError::Container(diagnostic(format!(
                        "unmanaged workload occupies {}",
                        desired.container.name.value
                    ))));
                }
            }
            if observation.is_none() {
                return Ok(Outcome {
                    project_root: project.root,
                    container_action: ContainerAction::Checked,
                    warnings: preflight.warnings,
                    attachment: AttachmentOutcome::Skipped,
                });
            }
            let transition = match invocation.operation {
                Operation::Stop => Transition::Stop,
                Operation::Remove => Transition::Remove,
                _ => unreachable!(),
            };
            let action = container_action(&transition);
            adapters
                .workload
                .apply(&transition, &desired)
                .map_err(|error| ReconcileError::Container(diagnostic(error.message)))?;
            Ok(Outcome {
                project_root: project.root,
                container_action: action,
                warnings: preflight.warnings,
                attachment: AttachmentOutcome::Skipped,
            })
        }
        Operation::Attach | Operation::Update { .. } => {
            if is_update {
                desired.packages.lock_policy = LockPolicy::Advance;
            }
            adapters
                .workload
                .ensure_image(&desired.container.image)
                .map_err(|error| ReconcileError::Image(diagnostic(error.message)))?;
            let observation = adapters
                .workload
                .observe(&desired.container.name)
                .map_err(|error| ReconcileError::Container(diagnostic(error.message)))?;
            let observation = observation.map(|mut observation| {
                if observation.managed
                    && (observation.labels.project_identity
                        != desired.container.labels.project_identity
                        || observation.labels.schema_version != 1)
                {
                    observation.managed = false;
                }
                observation
            });
            let mut lifecycle = LifecycleDecider::new();
            let transition = lifecycle
                .decide(&desired, observation.as_ref())
                .map_err(|_| {
                    ReconcileError::Container(diagnostic("could not decide workload lifecycle"))
                })?;
            if matches!(transition, Transition::Collision) {
                return Err(ReconcileError::Container(diagnostic(format!(
                    "unmanaged workload occupies {}",
                    desired.container.name.value
                ))));
            }
            let action = container_action(&transition);
            let handle = adapters
                .workload
                .apply(&transition, &desired)
                .map_err(|error| ReconcileError::Container(diagnostic(error.message)))?;

            let mut package_planner = PackagePlanner::new();
            let mut package_plan = package_planner
                .plan(&desired.packages, &project.state_paths)
                .map_err(|_| ReconcileError::Nix(diagnostic("could not create package plan")))?;
            package_plan.focus = update_package;
            adapters
                .workspace
                .write_generated(
                    package_plan.generated_metadata.clone(),
                    package_plan.flake_contents.clone(),
                )
                .map_err(|error| ReconcileError::Nix(diagnostic(error.message)))?;
            adapters
                .packages
                .reconcile(&package_plan, &handle, adapters.workload)
                .map_err(|error| ReconcileError::Nix(diagnostic(error.message)))?;

            let attachment = if matches!(invocation.operation, Operation::Attach) {
                let shell_check = adapters
                    .workload
                    .exec(
                        &handle,
                        ProcessCommand {
                            executable: "sh".into(),
                            arguments: vec![
                                "-lc".into(),
                                format!("command -v -- {}", effective.shell.executable).into(),
                            ],
                        },
                    )
                    .map_err(|error| ReconcileError::Attach(diagnostic(error.message)))?;
                if !shell_check.status.success() {
                    return Err(ReconcileError::Attach(diagnostic(format!(
                        "shell is not installed in the workload: {}",
                        effective.shell.executable
                    ))));
                }
                adapters
                    .terminal
                    .attach(&handle, &effective.shell)
                    .map_err(|error| ReconcileError::Attach(diagnostic(error.message)))?
            } else {
                AttachmentOutcome::Skipped
            };
            Ok(Outcome {
                project_root: project.root,
                container_action: action,
                warnings: preflight.warnings,
                attachment,
            })
        }
    }
}

fn container_action(transition: &Transition) -> ContainerAction {
    match transition {
        Transition::Create => ContainerAction::Created,
        Transition::Reuse => ContainerAction::ReusedRunning,
        Transition::Start => ContainerAction::Restarted,
        Transition::Recreate => ContainerAction::Recreated,
        Transition::Stop => ContainerAction::Stopped,
        Transition::Remove => ContainerAction::Removed,
        Transition::Collision | Transition::Noop => ContainerAction::Checked,
    }
}

fn diagnostic(message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        category: "reconciliation".to_owned(),
        message: message.into(),
        remediation: String::new(),
    }
}
