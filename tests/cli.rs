#![cfg(unix)]

#[path = "support/cli.rs"]
mod support;

use std::fs;

use support::{Cli, assert_failure, assert_success, stderr};

#[test]
fn remove_returns_early_but_still_stops_on_drop() {
    let cli = Cli::new();
    assert_success(&cli.run(&["remove"], &[]));
    assert_eq!(cli.calls("id"), [["-un"], ["-u"], ["-g"]]);
    assert_eq!(cli.phases(), ["remove", "stop"]);
    assert_eq!(
        cli.calls("docker"),
        [
            vec!["rm".into(), cli.name()],
            vec!["container".into(), "stop".into(), cli.name()]
        ]
    );
    assert!(!cli.workspace.join(".spawnbx").exists());
}

#[test]
fn remove_failure_stops_later_work_and_cleanup_failure_is_ignored() {
    let cli = Cli::new();
    assert_failure(
        &cli.run(&["remove"], &[("FAKE_FAIL_PHASE", "remove")]),
        "fake remove failure",
    );
    assert_eq!(cli.phases(), ["remove", "stop"]);
    let cli = Cli::new();
    assert_success(&cli.run(&["remove"], &[("FAKE_FAIL_PHASE", "stop")]));
    assert_eq!(cli.phases(), ["remove", "stop"]);
}

#[test]
fn running_container_inspects_then_creates_home_and_updates_without_attach() {
    let cli = Cli::new();
    assert_success(&cli.run(&["update"], &[("FAKE_INSPECT", " \ttrue\r\n")]));
    assert_eq!(
        cli.events(),
        [
            "id:-un",
            "id:-u",
            "id:-g",
            "inspect:absent",
            "user:present",
            "path:present",
            "profile:present",
            "upgrade:present",
            "stop:present"
        ]
    );
    let calls = cli.calls("docker");
    assert_eq!(
        calls[0],
        [
            "container",
            "inspect",
            "--format",
            "{{.State.Running}}",
            &cli.name()
        ]
    );
    assert_eq!(
        calls[4],
        [
            "exec",
            &cli.name(),
            "nix",
            "profile",
            "upgrade",
            "'.*'",
            "--profile",
            "/nix/var/nix/profiles/default"
        ]
    );
    assert_eq!(calls[5], ["container", "stop", &cli.name()]);
}

#[test]
fn every_inspect_success_other_than_trimmed_true_starts_existing_container() {
    for running in ["false", "True", "", "true false", "\u{a0}true\u{a0}"] {
        let cli = Cli::new();
        assert_success(&cli.run(&["update"], &[("FAKE_INSPECT", running)]));
        assert_eq!(
            cli.phases(),
            [
                "inspect", "start", "user", "path", "profile", "upgrade", "stop"
            ]
        );
        assert_eq!(cli.calls("docker")[1], ["container", "start", &cli.name()]);
        assert_eq!(cli.events()[4], "start:present");
    }
}

#[test]
fn missing_container_has_exact_create_argv_and_mounts() {
    let cli = Cli::new();
    assert_success(&cli.run(&["update"], &[("FAKE_INSPECT", "missing")]));
    let workspace_mount = format!("{}:/workspace", cli.workspace.display());
    let home_mount = format!("{}/.spawnbx/home:/home/spawnbx", cli.workspace.display());
    let image = if cfg!(debug_assertions) {
        "spawnbx:latest"
    } else {
        "ghcr.io/nthcristian/spawnbx:latest"
    };
    assert_eq!(
        cli.calls("docker")[1],
        [
            "run",
            "--detach",
            "--name",
            &cli.name(),
            "--workdir",
            "/workspace",
            "--env",
            "HOME=/home/spawnbx",
            "--volume",
            &workspace_mount,
            "--volume",
            &home_mount,
            image,
            "sleep",
            "infinity"
        ]
    );
    assert_eq!(cli.events()[3..5], ["inspect:absent", "create:present"]);
}

#[test]
fn missing_container_creates_wayland_pipewire_and_amd_gpu_integrations() {
    let cli = Cli::new();
    let xauthority = cli.workspace.join("xauthority");
    fs::write(&xauthority, "cookie").unwrap();
    assert_success(&cli.run(
        &["--wayland", "--pipewire", "--gpu", "update"],
        &[
            ("FAKE_INSPECT", "missing"),
            ("XDG_RUNTIME_DIR", "/run/user/1234"),
            ("WAYLAND_DISPLAY", "wayland-7"),
            ("DISPLAY", ":0"),
            ("XAUTHORITY", xauthority.to_str().unwrap()),
        ],
    ));

    let workspace_mount = format!("{}:/workspace", cli.workspace.display());
    let home_mount = format!("{}/.spawnbx/home:/home/spawnbx", cli.workspace.display());
    let image = if cfg!(debug_assertions) {
        "spawnbx:latest"
    } else {
        "ghcr.io/nthcristian/spawnbx:latest"
    };
    assert_eq!(
        cli.calls("docker")[1],
        [
            "run",
            "--detach",
            "--name",
            &cli.name(),
            "--workdir",
            "/workspace",
            "--env",
            "HOME=/home/spawnbx",
            "--volume",
            &workspace_mount,
            "--volume",
            &home_mount,
            "--env",
            "XDG_RUNTIME_DIR=/run/user/1234",
            "--env",
            "WAYLAND_DISPLAY=wayland-7",
            "--volume",
            "/run/user/1234/wayland-7:/run/user/1234/wayland-7",
            "--env",
            "DISPLAY=:0",
            "--volume",
            "/tmp/.X11-unix:/tmp/.X11-unix",
            "--env",
            "XAUTHORITY=/tmp/.Xauthority",
            "--volume",
            &format!("{}:/tmp/.Xauthority:ro", xauthority.display()),
            "--env",
            "PIPEWIRE_REMOTE=pipewire-0",
            "--volume",
            "/run/user/1234/pipewire-0:/run/user/1234/pipewire-0",
            "--device",
            "/dev/dri",
            image,
            "sleep",
            "infinity"
        ]
    );
}

#[test]
fn missing_wayland_environment_prevents_container_creation() {
    let cli = Cli::new();
    assert_failure(
        &cli.run(&["--wayland", "update"], &[("FAKE_INSPECT", "missing")]),
        "XDG_RUNTIME_DIR",
    );
    assert_eq!(cli.phases(), ["inspect", "stop"]);
}

#[test]
fn missing_xwayland_environment_prevents_container_creation() {
    let cli = Cli::new();
    assert_failure(
        &cli.run(
            &["--wayland", "update"],
            &[
                ("FAKE_INSPECT", "missing"),
                ("XDG_RUNTIME_DIR", "/run/user/1234"),
                ("WAYLAND_DISPLAY", "wayland-7"),
            ],
        ),
        "DISPLAY",
    );
    assert_eq!(cli.phases(), ["inspect", "stop"]);
}

#[test]
fn wayland_uses_home_xauthority_when_the_environment_variable_is_unavailable() {
    let cli = Cli::new();
    let xauthority = cli.workspace.join(".Xauthority");
    fs::write(&xauthority, "cookie").unwrap();
    assert_success(&cli.run(
        &["--wayland", "update"],
        &[
            ("FAKE_INSPECT", "missing"),
            ("XDG_RUNTIME_DIR", "/run/user/1234"),
            ("WAYLAND_DISPLAY", "wayland-7"),
            ("DISPLAY", ":0"),
            ("HOME", cli.workspace.to_str().unwrap()),
        ],
    ));

    assert!(
        cli.calls("docker")[1].contains(&format!("{}:/tmp/.Xauthority:ro", xauthority.display()))
    );
}

#[test]
fn wayland_allows_socket_only_xwayland_when_no_authority_file_exists() {
    let cli = Cli::new();
    assert_success(&cli.run(
        &["--wayland", "update"],
        &[
            ("FAKE_INSPECT", "missing"),
            ("XDG_RUNTIME_DIR", "/run/user/1234"),
            ("WAYLAND_DISPLAY", "wayland-7"),
            ("DISPLAY", ":0"),
        ],
    ));

    assert!(
        !cli.calls("docker")[1]
            .iter()
            .any(|argument| argument.starts_with("XAUTHORITY="))
    );
}

#[test]
fn completed_inspect_errors_still_create_home_before_returning() {
    for env in [
        [("FAKE_FAIL_PHASE", "inspect")],
        [("FAKE_INSPECT", "wrong-case")],
    ] {
        let cli = Cli::new();
        assert_failure(&cli.run(&["update"], &env), "inspect");
        assert_eq!(cli.phases(), ["inspect", "stop"]);
        assert!(cli.workspace.join(".spawnbx/home").is_dir());
        assert_eq!(cli.events()[3..], ["inspect:absent", "stop:present"]);
    }
}

#[test]
fn inspect_launch_failure_does_not_create_home() {
    let cli = Cli::new();
    fs::remove_file(cli.bin.join("docker")).unwrap();
    assert_failure(&cli.run(&["update"], &[]), "No such file");
    assert!(!cli.workspace.join(".spawnbx").exists());
    assert!(cli.calls("docker").is_empty());
}

#[test]
fn home_creation_failure_happens_after_inspect_and_prevents_start() {
    let cli = Cli::new();
    fs::write(cli.workspace.join(".spawnbx"), "not a directory").unwrap();
    assert_failure(
        &cli.run(&["update"], &[("FAKE_INSPECT", "false")]),
        "directory",
    );
    assert_eq!(cli.phases(), ["inspect", "stop"]);
}

#[test]
fn startup_failures_prevent_setup_but_still_stop() {
    for (inspect, phase) in [("false", "start"), ("missing", "create")] {
        let cli = Cli::new();
        assert_failure(
            &cli.run(
                &["update"],
                &[("FAKE_INSPECT", inspect), ("FAKE_FAIL_PHASE", phase)],
            ),
            &format!("fake {phase} failure"),
        );
        assert_eq!(cli.phases(), ["inspect", phase, "stop"]);
    }
}

#[test]
fn recreate_removes_then_starts_configures_and_attaches_successfully() {
    let cli = Cli::new();
    let output = cli.run(&["recreate"], &[("FAKE_INSPECT", "missing")]);
    assert_success(&output);
    assert_eq!(
        cli.phases(),
        [
            "remove", "inspect", "create", "user", "path", "profile", "attach", "stop"
        ]
    );
    assert_eq!(
        cli.events()[3..6],
        ["remove:absent", "inspect:absent", "create:present"]
    );
}

#[test]
fn attach_configures_the_environment_and_returns_after_the_shell_exits() {
    let cli = Cli::new();
    let output = cli.run(
        &[],
        &[
            ("FAKE_ATTACH_STDOUT", "interactive output\n"),
            ("FAKE_ATTACH_STDERR", "interactive stderr\n"),
        ],
    );
    assert_success(&output);
    assert_eq!(output.stdout, b"interactive output\n");
    assert!(stderr(&output).contains("interactive stderr"));
    assert_eq!(
        cli.phases(),
        ["inspect", "user", "path", "profile", "attach", "stop"]
    );
    assert_eq!(
        cli.calls("docker")[4],
        [
            "exec",
            "--interactive",
            "--tty",
            "--user",
            "1234:5678",
            &cli.name(),
            "fish"
        ]
    );
}

#[test]
fn stop_skips_lifecycle_configuration_and_attach() {
    let cli = Cli::new();
    assert_success(&cli.run(&["stop"], &[]));

    assert_eq!(cli.calls("id"), [["-un"], ["-u"], ["-g"]]);
    assert_eq!(cli.phases(), ["stop"]);
    assert_eq!(
        cli.calls("docker"),
        [vec!["container".into(), "stop".into(), cli.name()]]
    );
    assert!(!cli.workspace.join(".spawnbx").exists());

    let cli = Cli::new();
    assert_success(&cli.run(&["stop"], &[("FAKE_FAIL_PHASE", "stop")]));
    assert_eq!(cli.phases(), ["stop"]);
}

#[test]
fn attach_failure_returns_an_error_instead_of_panicking_and_stops() {
    let cli = Cli::new();
    let output = cli.run(&[], &[("FAKE_FAIL_PHASE", "attach")]);
    assert_failure(&output, "17");
    assert_eq!(output.status.code(), Some(1));
    assert!(!stderr(&output).contains("not yet implemented"));
    assert_eq!(
        cli.phases(),
        ["inspect", "user", "path", "profile", "attach", "stop"]
    );
}

#[test]
fn setup_scripts_are_passed_as_data_with_positional_identity_not_executed() {
    let cli = Cli::new();
    let username = "$(touch SHOULD_NOT_EXIST); hostile user";
    assert_success(&cli.run(&["update"], &[("FAKE_USERNAME", username)]));
    let calls = cli.calls("docker");
    assert_eq!(&calls[1][..5], ["exec", &cli.name(), "sh", "-eu", "-c"]);
    assert!(calls[1][5].contains("username=\"$1\""));
    assert!(!calls[1][5].contains(username));
    assert_eq!(&calls[1][6..], ["--", username, "1234", "5678"]);
    assert_eq!(&calls[2][..5], ["exec", &cli.name(), "sh", "-eu", "-c"]);
    assert!(calls[2][5].contains("/etc/profile.d/spawnbx-nix.sh"));
    assert_eq!(calls[2].len(), 6);
    assert_eq!(
        calls[3],
        [
            "exec",
            &cli.name(),
            "nix",
            "profile",
            "list",
            "--json",
            "--profile",
            "/nix/var/nix/profiles/default"
        ]
    );
    assert!(!cli.workspace.join("SHOULD_NOT_EXIST").exists());
}

#[test]
fn plugin_nonzero_statuses_halt_all_later_phases() {
    let phases = ["inspect", "user", "path", "profile", "upgrade", "install"];
    for (index, phase) in phases.iter().enumerate().skip(1) {
        let cli = Cli::new();
        cli.settings("name: character\npackages: [hello]\n");
        assert_failure(
            &cli.run(&["update"], &[("FAKE_FAIL_PHASE", phase)]),
            &format!("fake {phase} failure"),
        );
        let mut expected = phases[..=index].to_vec();
        expected.push("stop");
        assert_eq!(cli.phases(), expected);
    }
}

#[test]
fn executor_decodes_both_streams_strictly_before_plugins_continue() {
    for key in ["FAKE_BAD_STDOUT_PHASE", "FAKE_BAD_STDERR_PHASE"] {
        let cli = Cli::new();
        assert_failure(&cli.run(&["update"], &[(key, "user")]), "utf-8");
        assert_eq!(cli.phases(), ["inspect", "user", "stop"]);
    }
}

#[test]
fn signaled_executor_is_not_success_and_cleanup_still_runs() {
    let cli = Cli::new();
    let output = cli.run(&["update"], &[("FAKE_SIGNAL_PHASE", "user")]);
    assert!(!output.status.success(), "{}", stderr(&output));
    assert_eq!(cli.phases(), ["inspect", "user", "stop"]);
}

#[test]
fn package_updates_preserve_order_and_install_sorted_missing_packages() {
    let cli = Cli::new();
    cli.settings("name: character\npackages: [zlib, hello, alpha, alpha]\n");
    let profile = r#"{"elements":{"one":{"attrPath":"legacyPackages.x86_64-linux.hello"}}}"#;
    assert_success(&cli.run(&["update", "hello", "hello"], &[("FAKE_PROFILE", profile)]));
    let calls = cli.calls("docker");
    for call in &calls[4..6] {
        assert_eq!(
            call,
            &[
                "exec",
                &cli.name(),
                "nix",
                "profile",
                "upgrade",
                "hello",
                "--profile",
                "/nix/var/nix/profiles/default"
            ]
        );
    }
    assert_eq!(
        calls[6],
        [
            "exec",
            &cli.name(),
            "nix",
            "profile",
            "install",
            "--profile",
            "/nix/var/nix/profiles/default",
            "nixpkgs#alpha",
            "nixpkgs#zlib"
        ]
    );
    assert_eq!(
        cli.phases(),
        [
            "inspect", "user", "path", "profile", "upgrade", "upgrade", "install", "stop"
        ]
    );
}

#[test]
fn missing_named_update_keeps_prior_upgrade_but_skips_install_and_attach() {
    let cli = Cli::new();
    cli.settings("name: character\npackages: [new]\n");
    let profile = r#"{"elements":{"one":{"attrPath":"hello"}}}"#;
    assert_failure(
        &cli.run(
            &["update", "hello", "missing", "hello"],
            &[("FAKE_PROFILE", profile)],
        ),
        "missing",
    );
    assert_eq!(
        cli.phases(),
        ["inspect", "user", "path", "profile", "upgrade", "stop"]
    );
}

#[test]
fn save_happens_in_child_cwd_before_any_docker_work() {
    let cli = Cli::new();
    assert_success(&cli.run(
        &["--save", "--shell", "zsh", "--packages", "hello", "remove"],
        &[],
    ));
    let saved = fs::read_to_string(cli.workspace.join(".spawnbx.yml")).unwrap();
    assert!(saved.contains("shell: zsh"));
    assert!(saved.contains("- hello"));
    assert_eq!(
        saved,
        fs::read_to_string(cli.records.join("first-docker-settings")).unwrap()
    );
    assert_eq!(cli.phases(), ["remove", "stop"]);
}

#[test]
fn identity_failures_stop_lookup_and_never_construct_docker() {
    for (key, value, expected_calls) in [
        ("FAKE_ID_FAIL", "-u", 2),
        ("FAKE_ID_BAD_UTF8", "-un", 1),
        ("FAKE_USERNAME", "", 1),
        ("FAKE_UID", "not-a-number", 2),
        ("FAKE_GID", "4294967296", 3),
    ] {
        let cli = Cli::new();
        let output = cli.run(&["remove"], &[(key, value)]);
        assert!(!output.status.success(), "{}", stderr(&output));
        assert_eq!(cli.calls("id").len(), expected_calls);
        assert!(cli.calls("docker").is_empty());
        assert!(!cli.workspace.join(".spawnbx").exists());
    }
}

#[test]
fn id_launch_failure_never_constructs_docker() {
    let cli = Cli::new();
    fs::remove_file(cli.bin.join("id")).unwrap();
    assert_failure(&cli.run(&["remove"], &[]), "No such file");
    assert!(cli.calls("docker").is_empty());
}

#[test]
fn logging_defaults_to_concise_info_and_respects_filters() {
    let cli = Cli::new();
    let default_output = cli
        .command()
        .env_remove("RUST_LOG")
        .arg("update")
        .output()
        .unwrap();
    assert_success(&default_output);
    let default_stderr = stderr(&default_output);
    assert!(default_stderr.contains("Configuring environment"));
    assert!(default_stderr.contains("Updating all Nix packages"));
    assert!(default_stderr.contains("stopping container"));
    assert!(!default_stderr.contains("/etc/profile.d/spawnbx-nix.sh"));
    assert!(!default_stderr.contains("username=\"$1\""));
    assert!(default_output.stdout.is_empty());

    let off_output = cli.run(&["update"], &[]);
    assert_success(&off_output);
    assert!(stderr(&off_output).is_empty());

    let invalid_filter_output = cli
        .command()
        .env("RUST_LOG", "[")
        .arg("update")
        .output()
        .unwrap();
    assert_success(&invalid_filter_output);
    assert!(stderr(&invalid_filter_output).contains("Configuring environment"));
}
