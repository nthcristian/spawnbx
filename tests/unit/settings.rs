use super::*;
use std::{io::ErrorKind, os::unix::process::ExitStatusExt, process::ExitStatus};

fn host() -> HostIdentity {
    HostIdentity {
        username: "developer".into(),
        uid: 1234,
        gid: 5678,
    }
}

fn state() -> EnvironmentState {
    assemble_environment_state(
        serde_yaml::from_str("{}").unwrap(),
        Path::new("/work/project"),
        host(),
    )
    .unwrap()
}

#[test]
fn absent_fields_default_and_host_identity_is_preserved() {
    let state = state();
    assert_eq!(state.container_name_prefix, "project");
    assert_eq!(state.shell_program, "bash");
    assert!(state.package_names.is_empty());
    assert!(!state.wayland);
    assert!(!state.pipewire);
    assert!(!state.gpu);
    assert_eq!(state.workspace_root, "/work/project");
    assert_eq!(state.host_username, "developer");
    assert_eq!((state.host_uid, state.host_gid), (1234, 5678));
}

#[test]
fn configured_fields_are_used_without_validation_or_package_deduplication() {
    let settings =
        serde_yaml::from_str(
            "name: ''\nshell: ''\npackages: [git, git, curl]\nwayland: true\npipewire: true\ngpu: true\n",
        )
        .unwrap();
    let state = assemble_environment_state(settings, Path::new("/"), host()).unwrap();
    assert_eq!(state.container_name_prefix, "");
    assert_eq!(state.shell_program, "");
    assert_eq!(state.package_names, ["git", "git", "curl"]);
    assert!(state.wayland);
    assert!(state.pipewire);
    assert!(state.gpu);
    assert_eq!(state.workspace_root, "/");
}

#[test]
fn workspace_without_name_fails_only_when_name_is_not_configured() {
    let settings = serde_yaml::from_str("{}").unwrap();
    assert!(matches!(
        assemble_environment_state(settings, Path::new("/"), host()),
        Err(EnvironmentStateStoreError::StateLoadFailed { .. })
    ));
}

#[test]
fn yaml_unknown_fields_are_ignored_and_null_fields_use_defaults() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(SETTINGS_FILE);
    std::fs::write(
        &path,
        "name: null\nshell: null\npackages: null\nunknown: {nested: true}\n",
    )
    .unwrap();
    let state = assemble_environment_state(
        load_settings(&path).unwrap(),
        Path::new("/work/project"),
        host(),
    )
    .unwrap();
    assert_eq!(state.container_name_prefix, "project");
    assert_eq!(state.shell_program, "bash");
    assert!(state.package_names.is_empty());
}

#[test]
fn characterization_all_read_failures_share_missing_file_category_and_keep_io_source() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing.yml");
    let invalid_utf8 = directory.path().join("invalid-utf8.yml");
    std::fs::write(&invalid_utf8, [0xff]).unwrap();
    for (path, expected_kind) in [
        (missing, ErrorKind::NotFound),
        (directory.path().to_owned(), ErrorKind::IsADirectory),
        (invalid_utf8, ErrorKind::InvalidData),
    ] {
        let EnvironmentStateStoreError::SettingsReadFailed {
            path: actual_path,
            source,
        } = load_settings(&path).unwrap_err()
        else {
            panic!("settings read failure changed category");
        };
        assert_eq!(actual_path, path);
        assert!(source.to_string().contains(&path.display().to_string()));
        assert_eq!(
            source.downcast_ref::<std::io::Error>().unwrap().kind(),
            expected_kind
        );
    }
}

#[test]
fn malformed_yaml_and_invalid_field_types_remain_load_failures() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(SETTINGS_FILE);
    for yaml in ["packages: [", "packages: not-a-list", "shell: [bash]"] {
        std::fs::write(&path, yaml).unwrap();
        let EnvironmentStateStoreError::StateLoadFailed { source } =
            load_settings(&path).unwrap_err()
        else {
            panic!("YAML failure changed category");
        };
        assert!(source.downcast_ref::<serde_yaml::Error>().is_some());
        assert!(source.to_string().contains(&path.display().to_string()));
    }
}

#[test]
fn saved_yaml_round_trips_environment_settings() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(SETTINGS_FILE);
    let mut state = state();
    state.container_name_prefix = "my-project".into();
    state.shell_program = "/bin/zsh".into();
    state.package_names = vec!["curl".into(), "git".into(), "curl".into()];
    state.wayland = true;
    state.pipewire = true;
    state.gpu = true;
    save_settings(&path, &serialize_settings(&state).unwrap()).unwrap();
    let yaml = std::fs::read_to_string(&path).unwrap();
    let document: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
    let mapping = document.as_mapping().unwrap();
    assert_eq!(mapping.len(), 6);
    assert_eq!(document["name"].as_str(), Some("my-project"));
    assert_eq!(document["shell"].as_str(), Some("/bin/zsh"));
    assert_eq!(
        document["packages"],
        serde_yaml::to_value(&state.package_names).unwrap()
    );
    assert_eq!(document["wayland"].as_bool(), Some(true));
    assert_eq!(document["pipewire"].as_bool(), Some(true));
    assert_eq!(document["gpu"].as_bool(), Some(true));
    let settings = load_settings(&path).unwrap();
    assert_eq!(
        settings.container_name_prefix.as_deref(),
        Some("my-project")
    );
    assert_eq!(settings.shell_program.as_deref(), Some("/bin/zsh"));
    assert_eq!(settings.package_names, Some(state.package_names));
    assert_eq!(settings.wayland, Some(true));
    assert_eq!(settings.pipewire, Some(true));
    assert_eq!(settings.gpu, Some(true));
}

#[test]
fn write_failure_remains_save_failed_with_path_and_io_source() {
    let directory = tempfile::tempdir().unwrap();
    let EnvironmentStateStoreError::SettingsSaveFailed { source } =
        save_settings(directory.path(), "{}").unwrap_err()
    else {
        panic!("write failure changed category");
    };
    assert!(
        source
            .to_string()
            .contains(&directory.path().display().to_string())
    );
    assert!(source.downcast_ref::<std::io::Error>().is_some());
}

#[test]
fn characterization_save_targets_current_directory_not_state_workspace_root() {
    let cwd = tempfile::tempdir().unwrap();
    let state_root = tempfile::tempdir().unwrap();
    run_store_child(cwd.path(), state_root.path(), "save");
    assert!(cwd.path().join(SETTINGS_FILE).is_file());
    assert!(!state_root.path().join(SETTINGS_FILE).exists());
}

#[test]
fn characterization_save_current_directory_failure_is_load_failed() {
    let cwd = tempfile::tempdir().unwrap();
    let state_root = tempfile::tempdir().unwrap();
    run_store_child(cwd.path(), state_root.path(), "missing-cwd");
    assert!(!cwd.path().exists());
    assert!(!state_root.path().join(SETTINGS_FILE).exists());
}

fn run_store_child(cwd: &Path, state_root: &Path, mode: &str) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "adapters::project_settings_store::tests::settings_store_child",
            "--nocapture",
        ])
        .current_dir(cwd)
        .env("SPAWNBX_SETTINGS_TEST_CHILD", mode)
        .env("SPAWNBX_SETTINGS_TEST_ROOT", state_root)
        .output()
        .unwrap();
    assert!(output.status.success(), "child failed: {output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
}

#[test]
fn settings_store_child() {
    let Ok(mode) = std::env::var("SPAWNBX_SETTINGS_TEST_CHILD") else {
        return;
    };
    let mut state = state();
    state.workspace_root = std::env::var("SPAWNBX_SETTINGS_TEST_ROOT").unwrap();
    match mode.as_str() {
        "save" => ProjectSettingsStore.save(&state).unwrap(),
        "missing-cwd" => {
            // Only this isolated child loses its cwd; the parent test process is untouched.
            std::fs::remove_dir(std::env::current_dir().unwrap()).unwrap();
            let EnvironmentStateStoreError::StateLoadFailed { source } =
                ProjectSettingsStore.save(&state).unwrap_err()
            else {
                panic!("save cwd lookup failure changed category");
            };
            assert!(source.to_string().contains("current directory"));
            assert!(source.downcast_ref::<std::io::Error>().is_some());
        }
        _ => panic!("unknown child mode"),
    }
}

fn output(status: i32, stdout: &[u8], stderr: &[u8]) -> Output {
    Output {
        status: ExitStatus::from_raw(status),
        stdout: stdout.to_vec(),
        stderr: stderr.to_vec(),
    }
}

#[test]
fn host_identity_output_trims_whitespace_without_other_validation() {
    assert_eq!(
        decode_identity_output("-un", output(0, b" \tdeveloper\n", b"ignored")).unwrap(),
        "developer"
    );
    assert_eq!(
        decode_identity_output("-u", output(0, b"not-a-number", b"")).unwrap(),
        "not-a-number"
    );
}

#[test]
fn host_identity_decode_failure_keeps_utf8_source() {
    let EnvironmentStateStoreError::StateLoadFailed { source } =
        decode_identity_output("-un", output(0, &[0xff], b"")).unwrap_err()
    else {
        panic!("identity decode failure changed category");
    };
    assert!(
        source
            .downcast_ref::<std::string::FromUtf8Error>()
            .is_some()
    );
    assert!(source.to_string().contains("id -un"));
}

#[test]
fn host_identity_nonzero_signal_and_empty_results_remain_load_failures() {
    for result in [
        output(7 << 8, &[0xff], b" denied\n"),
        output(9, b"developer", b"killed"),
        output(0, b" \n\t", b""),
    ] {
        let EnvironmentStateStoreError::StateLoadFailed { source } =
            decode_identity_output("-un", result).unwrap_err()
        else {
            panic!("identity failure changed category");
        };
        assert!(source.to_string().contains("id -un"));
        assert!(
            source
                .downcast_ref::<std::string::FromUtf8Error>()
                .is_none()
        );
    }
}
