mod adapters;
mod cli;
mod reconcile;

use adapters::{DockerCliAdapter, NixCliAdapter, OsTerminalAdapter, OsWorkspaceAdapter};
use adapters::{LinuxHostAdapter, SystemProcessAdapter};
use cli::Cli;
use reconcile::{Adapters, Operation};

fn main() {
    let cli = match Cli::parse(std::env::args_os()) {
        Ok(cli) => cli,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    let mut workspace = OsWorkspaceAdapter::new();
    let mut host = LinuxHostAdapter::new();
    let mut docker = DockerCliAdapter::new(Box::new(SystemProcessAdapter::new()));
    let mut packages = NixCliAdapter::new();
    let mut terminal = OsTerminalAdapter::new(Box::new(SystemProcessAdapter::new()));
    let mut adapters = Adapters {
        workspace: &mut workspace,
        host: &mut host,
        workload: &mut docker,
        packages: &mut packages,
        terminal: &mut terminal,
    };

    let is_doctor = matches!(cli.invocation.operation, Operation::Doctor);
    match reconcile::run(cli.invocation, &mut adapters) {
        Ok(outcome) => {
            for warning in outcome.warnings {
                eprintln!("warning: {}", warning.message);
            }
            if is_doctor {
                println!(
                    "doctor: ok ({})",
                    outcome.project_root.canonical_path.display()
                );
            }
        }
        Err(error) => {
            eprintln!("{}", render_error(&error));
            std::process::exit(1);
        }
    }
}

fn render_error(error: &reconcile::ReconcileError) -> String {
    match error {
        reconcile::ReconcileError::Preflight(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.category, diagnostic.message))
            .collect::<Vec<_>>()
            .join("\n"),
        reconcile::ReconcileError::Configuration(diagnostic)
        | reconcile::ReconcileError::UnsafeProjectState(diagnostic)
        | reconcile::ReconcileError::Image(diagnostic)
        | reconcile::ReconcileError::Container(diagnostic)
        | reconcile::ReconcileError::Nix(diagnostic)
        | reconcile::ReconcileError::Attach(diagnostic) => {
            if diagnostic.remediation.is_empty() {
                format!("{}: {}", diagnostic.category, diagnostic.message)
            } else {
                format!(
                    "{}: {}\nremediation: {}",
                    diagnostic.category, diagnostic.message, diagnostic.remediation
                )
            }
        }
    }
}
