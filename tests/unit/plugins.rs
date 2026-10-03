use std::{cell::RefCell, collections::VecDeque, error::Error, io};

use super::*;
use crate::adapters::UserConfigurator;

const PROFILE: &str = "/nix/var/nix/profiles/default";
const EMPTY_PROFILE: &str = r#"{"elements":{}}"#;
const GIT_PROFILE: &str = r#"{"elements":{"git":{"attrPath":"packages.x86_64-linux.git"}}}"#;

#[derive(Default)]
struct RecordingExecutor {
    calls: RefCell<Vec<Vec<String>>>,
    outputs: RefCell<VecDeque<anyhow::Result<CommandOutput>>>,
}

impl RecordingExecutor {
    fn new(outputs: impl IntoIterator<Item = anyhow::Result<CommandOutput>>) -> Self {
        Self {
            outputs: RefCell::new(outputs.into_iter().collect()),
            ..Self::default()
        }
    }

    fn executor(&self) -> CommandExecutor<'_> {
        Box::new(|command| {
            self.calls
                .borrow_mut()
                .push(command.iter().map(|arg| (*arg).to_owned()).collect());
            self.outputs
                .borrow_mut()
                .pop_front()
                .expect("unexpected command")
        })
    }

    fn assert_calls(&self, expected: &[Vec<&str>]) {
        assert_eq!(*self.calls.borrow(), expected);
        assert!(self.outputs.borrow().is_empty(), "unconsumed outputs");
    }
}

fn output(stdout: &str) -> CommandOutput {
    CommandOutput {
        stdout: stdout.to_owned(),
        stderr: String::new(),
        exit_code: Some(0),
    }
}

fn state(packages: &[&str]) -> EnvironmentState {
    EnvironmentState {
        container_name_prefix: "test".to_owned(),
        workspace_root: "/unused".to_owned(),
        package_names: packages.iter().map(|name| (*name).to_owned()).collect(),
        wayland: false,
        pipewire: false,
        gpu: false,
        shell_program: "bash".to_owned(),
        host_username: "test-user".to_owned(),
        host_uid: 1234,
        host_gid: 5678,
    }
}

fn invocation(operation: CommandOperation) -> CommandInvocation {
    CommandInvocation {
        operation,
        additional_package_names: Vec::new(),
        requested_shell: None,
        wayland: false,
        pipewire: false,
        gpu: false,
        save_settings: false,
    }
}

fn update(names: &[&str]) -> CommandInvocation {
    invocation(CommandOperation::Update {
        package_names: names.iter().map(|name| (*name).to_owned()).collect(),
    })
}

fn inspection_commands() -> Vec<Vec<&'static str>> {
    vec![
        vec!["sh", "-eu", "-c", include_str!("../fixtures/nix_path.sh")],
        vec!["nix", "profile", "list", "--json", "--profile", PROFILE],
    ]
}

fn upgrade_command(package: &str) -> Vec<&str> {
    vec!["nix", "profile", "upgrade", package, "--profile", PROFILE]
}

#[test]
fn nix_configures_exact_path_script_then_inspects_even_without_packages() {
    let recording = RecordingExecutor::new([Ok(output("")), Ok(output(EMPTY_PROFILE))]);
    NixConfigurator::configure(
        &recording.executor(),
        &state(&[]),
        &invocation(CommandOperation::Attach),
    )
    .unwrap();
    recording.assert_calls(&inspection_commands());
}

#[test]
fn missing_packages_are_sorted_deduplicated_and_installed_after_inspection() {
    let recording = RecordingExecutor::new([
        Ok(output("")),
        Ok(output(GIT_PROFILE)),
        Ok(output("ignored install output")),
    ]);
    NixConfigurator::configure(
        &recording.executor(),
        &state(&["zsh", "git", "curl", "zsh", "curl"]),
        &invocation(CommandOperation::Attach),
    )
    .unwrap();
    let mut commands = inspection_commands();
    commands.push(vec![
        "nix",
        "profile",
        "install",
        "--profile",
        PROFILE,
        "nixpkgs#curl",
        "nixpkgs#zsh",
    ]);
    recording.assert_calls(&commands);
}

#[test]
fn installed_profile_packages_do_not_trigger_install() {
    let recording = RecordingExecutor::new([Ok(output("")), Ok(output(GIT_PROFILE))]);
    NixConfigurator::configure(
        &recording.executor(),
        &state(&["git", "git"]),
        &invocation(CommandOperation::Attach),
    )
    .unwrap();
    recording.assert_calls(&inspection_commands());
}

#[test]
fn characterization_non_update_operations_still_configure_without_upgrading() {
    for operation in [
        CommandOperation::Attach,
        CommandOperation::Remove,
        CommandOperation::Stop,
        CommandOperation::Recreate,
    ] {
        let recording = RecordingExecutor::new([Ok(output("")), Ok(output(GIT_PROFILE))]);
        NixConfigurator::configure(
            &recording.executor(),
            &state(&["git"]),
            &invocation(operation),
        )
        .unwrap();
        recording.assert_calls(&inspection_commands());
    }
}

#[test]
fn characterization_update_all_preserves_literal_quoted_pattern_and_original_snapshot() {
    let recording = RecordingExecutor::new([
        Ok(output("")),
        Ok(output(EMPTY_PROFILE)),
        Ok(output(GIT_PROFILE)),
        Ok(output("")),
    ]);
    NixConfigurator::configure(&recording.executor(), &state(&["git"]), &update(&[])).unwrap();
    let mut commands = inspection_commands();
    commands.push(upgrade_command("'.*'"));
    commands.push(vec![
        "nix",
        "profile",
        "install",
        "--profile",
        PROFILE,
        "nixpkgs#git",
    ]);
    recording.assert_calls(&commands);
}

#[test]
fn named_updates_preserve_order_duplicates_and_pre_update_snapshot() {
    let profile = r#"{"elements":{"a":{"attrPath":"git"},"b":{"attrPath":"curl"}}}"#;
    let recording = RecordingExecutor::new([
        Ok(output("")),
        Ok(output(profile)),
        Ok(output(EMPTY_PROFILE)),
        Ok(output("not JSON")),
        Ok(output("")),
        Ok(output("")),
    ]);
    NixConfigurator::configure(
        &recording.executor(),
        &state(&["git", "curl", "zsh"]),
        &update(&["git", "curl", "git"]),
    )
    .unwrap();
    let mut commands = inspection_commands();
    commands.extend([
        upgrade_command("git"),
        upgrade_command("curl"),
        upgrade_command("git"),
    ]);
    commands.push(vec![
        "nix",
        "profile",
        "install",
        "--profile",
        PROFILE,
        "nixpkgs#zsh",
    ]);
    recording.assert_calls(&commands);
}

#[test]
fn characterization_missing_named_update_keeps_prior_upgrades_and_stops_before_install() {
    for names in [vec!["missing", "git"], vec!["git", "missing", "git"]] {
        let mut outputs = vec![Ok(output("")), Ok(output(GIT_PROFILE))];
        let mut commands = inspection_commands();
        if names[0] == "git" {
            outputs.push(Ok(output("")));
            commands.push(upgrade_command("git"));
        }
        let recording = RecordingExecutor::new(outputs);
        let error =
            NixConfigurator::configure(&recording.executor(), &state(&["curl"]), &update(&names))
                .unwrap_err();
        let EnvironmentConfigurationError::ConfigurationFailed { source } = error;
        assert!(source.to_string().contains("missing isn't installed"));
        recording.assert_calls(&commands);
    }
}

#[test]
fn characterization_profile_parsing_is_permissive_and_only_strips_x86_64_prefixes() {
    let profile = r#"{"elements":{
        "active":{"active":true,"attrPath":"packages.x86_64-linux.git"},
        "inactive":{"active":false,"attrPath":"hidden"},
        "default":{"attrPath":"legacyPackages.x86_64-linux.curl"},
        "null-active":{"active":null,"attrPath":"zsh"},
        "string-active":{"active":"false","attrPath":"fish"},
        "number-active":{"active":0,"attrPath":"jq"},
        "other-architecture":{"attrPath":"packages.aarch64-linux.git"},
        "embedded-prefix":{"attrPath":"nested.packages.x86_64-linux.git"},
        "empty-attribute":{"attrPath":""},
        "duplicate":{"attrPath":"git"},
        "no-attribute":{"active":true},
        "numeric-attribute":{"attrPath":42},
        "null-attribute":{"attrPath":null},
        "null-element":null,
        "scalar-element":17,
        "array-element":[]
    },"ignored":true}"#;
    let packages = profile_packages(output(profile)).unwrap();
    assert_eq!(
        packages,
        [
            "",
            "curl",
            "fish",
            "git",
            "jq",
            "nested.packages.x86_64-linux.git",
            "packages.aarch64-linux.git",
            "zsh"
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    );
    assert!(profile_packages(output(EMPTY_PROFILE)).unwrap().is_empty());
}

#[test]
fn missing_elements_object_halts_updates_and_install() {
    for profile in [
        "{}",
        "null",
        "[]",
        "1",
        r#"{"elements":null}"#,
        r#"{"elements":[]}"#,
        r#"{"elements":"wrong"}"#,
    ] {
        let recording = RecordingExecutor::new([Ok(output("")), Ok(output(profile))]);
        let error =
            NixConfigurator::configure(&recording.executor(), &state(&["git"]), &update(&[]))
                .unwrap_err();
        let EnvironmentConfigurationError::ConfigurationFailed { source } = error;
        assert!(source.to_string().contains("without elements"), "{profile}");
        recording.assert_calls(&inspection_commands());
    }
}

#[test]
fn malformed_profile_preserves_json_source_and_halts_updates_and_install() {
    let recording = RecordingExecutor::new([Ok(output("")), Ok(output("not JSON"))]);
    let error = NixConfigurator::configure(&recording.executor(), &state(&["git"]), &update(&[]))
        .unwrap_err();
    assert!(error.source().is_some());
    let EnvironmentConfigurationError::ConfigurationFailed { source } = error;
    assert!(source.to_string().contains("parse the system Nix profile"));
    assert!(source.downcast_ref::<serde_json::Error>().is_some());
    recording.assert_calls(&inspection_commands());
}

fn failed_output(case: usize) -> anyhow::Result<CommandOutput> {
    match case {
        0 => Err(anyhow::Error::from(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "transport denied",
        ))
        .context("executor transport")),
        1 => Ok(CommandOutput {
            exit_code: Some(7),
            stderr: "failure detail".to_owned(),
            ..output("")
        }),
        2 => Ok(CommandOutput {
            exit_code: None,
            stderr: "failure detail".to_owned(),
            ..output("")
        }),
        3 => Ok(CommandOutput {
            exit_code: Some(-1),
            stderr: "failure detail".to_owned(),
            ..output("")
        }),
        _ => unreachable!(),
    }
}

fn assert_failure(error: EnvironmentConfigurationError, case: usize, operation: &str) {
    assert!(error.source().is_some());
    let EnvironmentConfigurationError::ConfigurationFailed { source } = error;
    let message = source.to_string();
    assert!(message.contains(operation), "{message}");
    if case == 0 {
        let original = source
            .downcast_ref::<io::Error>()
            .expect("original I/O source");
        assert_eq!(original.kind(), io::ErrorKind::PermissionDenied);
        assert!(format!("{source:#}").contains("transport denied"));
        assert!(format!("{source:#}").contains("executor transport"));
    } else {
        let status = match case {
            1 => "exit code 7",
            2 => "no exit code",
            3 => "exit code -1",
            _ => unreachable!(),
        };
        assert!(message.contains(status), "{message}");
        assert!(message.contains("failure detail"), "{message}");
    }
}

#[test]
fn all_nix_execution_failures_preserve_context_and_halt_remaining_work() {
    let contexts = [
        "configure the system Nix PATH",
        "inspect the system Nix profile",
        "update Nix package git",
        "install Nix packages",
    ];
    let mut commands = inspection_commands();
    commands.push(upgrade_command("git"));
    commands.push(vec![
        "nix",
        "profile",
        "install",
        "--profile",
        PROFILE,
        "nixpkgs#curl",
    ]);
    for (phase, context) in contexts.iter().enumerate() {
        for case in 0..4 {
            let mut outputs = Vec::new();
            for prior in 0..phase {
                outputs.push(Ok(output(if prior == 1 { GIT_PROFILE } else { "" })));
            }
            outputs.push(failed_output(case));
            let recording = RecordingExecutor::new(outputs);
            let error = NixConfigurator::configure(
                &recording.executor(),
                &state(&["curl"]),
                &update(&["git"]),
            )
            .unwrap_err();
            assert_failure(error, case, context);
            recording.assert_calls(&commands[..=phase]);
        }
    }
}

#[test]
fn update_all_failure_stops_before_install() {
    for case in 0..4 {
        let recording = RecordingExecutor::new([
            Ok(output("")),
            Ok(output(EMPTY_PROFILE)),
            failed_output(case),
        ]);
        let error =
            NixConfigurator::configure(&recording.executor(), &state(&["git"]), &update(&[]))
                .unwrap_err();
        assert_failure(error, case, "update all Nix packages");
        let mut commands = inspection_commands();
        commands.push(upgrade_command("'.*'"));
        recording.assert_calls(&commands);
    }
}

#[test]
fn failed_named_upgrade_keeps_prior_progress_and_skips_later_upgrades() {
    let recording = RecordingExecutor::new([
        Ok(output("")),
        Ok(output(GIT_PROFILE)),
        Ok(output("")),
        failed_output(1),
    ]);
    let error = NixConfigurator::configure(
        &recording.executor(),
        &state(&["curl"]),
        &update(&["git", "git", "git"]),
    )
    .unwrap_err();
    assert_failure(error, 1, "update Nix package git");
    let mut commands = inspection_commands();
    commands.extend([upgrade_command("git"), upgrade_command("git")]);
    recording.assert_calls(&commands);
}

#[test]
fn checked_execution_returns_success_output_unchanged_including_stderr() {
    let recording = RecordingExecutor::new([Ok(CommandOutput {
        stdout: "stdout\n".to_owned(),
        stderr: "warning\n".to_owned(),
        exit_code: Some(0),
    })]);
    let result = execute_checked(
        &recording.executor(),
        &["command", "argument"],
        "test operation",
    )
    .unwrap();
    assert_eq!(result.stdout, "stdout\n");
    assert_eq!(result.stderr, "warning\n");
    assert_eq!(result.exit_code, Some(0));
    recording.assert_calls(&[vec!["command", "argument"]]);
}

#[test]
fn user_setup_preserves_exact_script_and_passes_untrusted_username_positionally() {
    let mut desired = state(&[]);
    desired.host_username = "user; $(touch /never-executed) ' \"\n".to_owned();
    let recording = RecordingExecutor::new([Ok(output("ignored"))]);
    UserConfigurator::configure(&recording.executor(), &desired, &update(&[])).unwrap();
    recording.assert_calls(&[vec![
        "sh",
        "-eu",
        "-c",
        include_str!("../fixtures/user_setup.sh"),
        "--",
        &desired.host_username,
        "1234",
        "5678",
    ]]);
}

#[test]
fn user_setup_accepts_zero_identity_and_preserves_command_failures() {
    let mut desired = state(&[]);
    desired.host_uid = 0;
    desired.host_gid = u32::MAX;
    for case in 0..4 {
        let recording = RecordingExecutor::new([failed_output(case)]);
        let error = UserConfigurator::configure(
            &recording.executor(),
            &desired,
            &invocation(CommandOperation::Attach),
        )
        .unwrap_err();
        assert_failure(error, case, "configure container user");
        recording.assert_calls(&[vec![
            "sh",
            "-eu",
            "-c",
            include_str!("../fixtures/user_setup.sh"),
            "--",
            "test-user",
            "0",
            "4294967295",
        ]]);
    }
}
