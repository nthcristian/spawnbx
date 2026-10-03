use std::{cell::RefCell, collections::BTreeSet, error::Error, io};

use super::*;
use crate::{
    ApplicationError,
    contracts::{
        EnvironmentConfigurationError, EnvironmentRuntimeError, EnvironmentStateStore,
        EnvironmentStateStoreError,
    },
    model::EnvironmentState,
};

struct RecordingStore {
    state: EnvironmentState,
    calls: RefCell<Vec<&'static str>>,
    saved: RefCell<Vec<EnvironmentState>>,
    fail_load: bool,
    fail_save: bool,
}

impl RecordingStore {
    fn new() -> Self {
        Self {
            state: EnvironmentState {
                container_name_prefix: "project".into(),
                workspace_root: "/workspace/project".into(),
                package_names: vec!["git".into(), "git".into(), "fish".into()],
                wayland: false,
                pipewire: false,
                gpu: false,
                shell_program: "bash".into(),
                host_username: "developer".into(),
                host_uid: 1000,
                host_gid: 1001,
            },
            calls: RefCell::default(),
            saved: RefCell::default(),
            fail_load: false,
            fail_save: false,
        }
    }
}

impl EnvironmentStateStore for RecordingStore {
    fn load(&self) -> Result<EnvironmentState, EnvironmentStateStoreError> {
        self.calls.borrow_mut().push("load");
        if self.fail_load {
            return Err(EnvironmentStateStoreError::StateLoadFailed {
                source: io::Error::new(io::ErrorKind::PermissionDenied, "settings denied").into(),
            });
        }
        Ok(self.state.clone())
    }

    fn save(&self, state: &EnvironmentState) -> Result<(), EnvironmentStateStoreError> {
        self.calls.borrow_mut().push("save");
        if self.fail_save {
            return Err(EnvironmentStateStoreError::SettingsSaveFailed {
                source: io::Error::new(io::ErrorKind::PermissionDenied, "write denied").into(),
            });
        }
        self.saved.borrow_mut().push(state.clone());
        Ok(())
    }
}

fn invocation() -> CommandInvocation {
    CommandInvocation {
        operation: CommandOperation::Attach,
        additional_package_names: Vec::new(),
        requested_shell: None,
        wayland: false,
        pipewire: false,
        gpu: false,
        save_settings: false,
    }
}

fn packages(state: &EnvironmentState) -> BTreeSet<&str> {
    state.package_names.iter().map(String::as_str).collect()
}

#[test]
fn resolution_preserves_loaded_fields_and_deduplicates_without_saving() {
    let store = RecordingStore::new();
    let state = resolve_environment_state(&invocation(), &store).unwrap();

    assert_eq!(
        state.container_name_prefix,
        store.state.container_name_prefix
    );
    assert_eq!(state.workspace_root, store.state.workspace_root);
    assert_eq!(state.shell_program, store.state.shell_program);
    assert_eq!(state.host_username, store.state.host_username);
    assert_eq!(state.host_uid, store.state.host_uid);
    assert_eq!(state.host_gid, store.state.host_gid);
    assert_eq!(packages(&state), BTreeSet::from(["fish", "git"]));
    assert_eq!(state.package_names.len(), 2);
    assert_eq!(*store.calls.borrow(), ["load"]);
    assert!(store.saved.borrow().is_empty());
}

#[test]
fn resolution_saves_the_merged_state_only_when_requested() {
    let store = RecordingStore::new();
    let request = CommandInvocation {
        additional_package_names: vec!["git".into(), "ripgrep".into(), "ripgrep".into()],
        requested_shell: Some("fish".into()),
        save_settings: true,
        ..invocation()
    };
    let state = resolve_environment_state(&request, &store).unwrap();

    assert_eq!(state.shell_program, "fish");
    assert_eq!(packages(&state), BTreeSet::from(["fish", "git", "ripgrep"]));
    assert_eq!(state.package_names.len(), 3);
    assert_eq!(*store.calls.borrow(), ["load", "save"]);
    let saved = store.saved.borrow();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].package_names, state.package_names);
    assert_eq!(saved[0].shell_program, state.shell_program);
    assert_eq!(saved[0].workspace_root, state.workspace_root);
    assert_eq!(saved[0].host_uid, state.host_uid);
}

#[test]
fn resolution_enables_requested_integrations_without_disabling_saved_settings() {
    let mut store = RecordingStore::new();
    store.state.wayland = true;
    let request = CommandInvocation {
        pipewire: true,
        gpu: true,
        save_settings: true,
        ..invocation()
    };

    let state = resolve_environment_state(&request, &store).unwrap();

    assert!(state.wayland);
    assert!(state.pipewire);
    assert!(state.gpu);
    let saved = store.saved.borrow();
    assert_eq!(saved.len(), 1);
    assert!(saved[0].wayland);
    assert!(saved[0].pipewire);
    assert!(saved[0].gpu);
}

#[test]
fn explicit_empty_shell_is_not_treated_as_a_missing_override() {
    let request = CommandInvocation {
        requested_shell: Some(String::new()),
        ..invocation()
    };
    let state = resolve_environment_state(&request, &RecordingStore::new()).unwrap();
    assert!(state.shell_program.is_empty());
}

#[test]
fn resolution_does_not_depend_on_operation() {
    for operation in [
        CommandOperation::Attach,
        CommandOperation::Remove,
        CommandOperation::Stop,
        CommandOperation::Recreate,
        CommandOperation::Update {
            package_names: vec!["not-an-additional-package".into()],
        },
    ] {
        let store = RecordingStore::new();
        let request = CommandInvocation {
            operation,
            save_settings: true,
            ..invocation()
        };
        let state = resolve_environment_state(&request, &store).unwrap();
        assert_eq!(packages(&state), BTreeSet::from(["fish", "git"]));
        assert_eq!(*store.calls.borrow(), ["load", "save"]);
    }
}

#[test]
fn load_failure_preserves_the_typed_source_and_prevents_save() {
    let store = RecordingStore {
        fail_load: true,
        ..RecordingStore::new()
    };
    let request = CommandInvocation {
        save_settings: true,
        ..invocation()
    };
    let error = resolve_environment_state(&request, &store).unwrap_err();

    assert!(matches!(
        &error,
        ApplicationError::StateResolutionFailed {
            source: EnvironmentStateStoreError::StateLoadFailed { .. }
        }
    ));
    assert!(error.source().unwrap().is::<EnvironmentStateStoreError>());
    assert!(format!("{:#}", anyhow::Error::new(error)).contains("settings denied"));
    assert_eq!(*store.calls.borrow(), ["load"]);
}

#[test]
fn save_failure_is_not_reported_as_successful_resolution() {
    let store = RecordingStore {
        fail_save: true,
        ..RecordingStore::new()
    };
    let request = CommandInvocation {
        save_settings: true,
        ..invocation()
    };
    let error = resolve_environment_state(&request, &store).unwrap_err();

    assert!(matches!(
        &error,
        ApplicationError::StateResolutionFailed {
            source: EnvironmentStateStoreError::SettingsSaveFailed { .. }
        }
    ));
    assert!(format!("{:#}", anyhow::Error::new(error)).contains("write denied"));
    assert_eq!(*store.calls.borrow(), ["load", "save"]);
}

#[test]
fn parse_errors_keep_the_parser_error_and_its_original_source() {
    let error = ApplicationError::from(InvocationParseError::ParseFailed {
        source: anyhow::anyhow!("invalid command input"),
    });

    assert!(matches!(
        error,
        ApplicationError::InvocationParseFailed { .. }
    ));
    assert!(error.source().unwrap().is::<InvocationParseError>());
    assert!(format!("{:#}", anyhow::Error::new(error)).contains("invalid command input"));
}

#[test]
fn runtime_and_plugin_errors_keep_the_same_application_category_and_sources() {
    let environment = ApplicationError::environment(EnvironmentRuntimeError::RuntimeFailure {
        source: io::Error::new(io::ErrorKind::NotFound, "docker unavailable").into(),
    });
    let plugin =
        ApplicationError::environment(EnvironmentConfigurationError::ConfigurationFailed {
            source: anyhow::anyhow!("package update failed"),
        });

    assert!(
        environment
            .source()
            .unwrap()
            .is::<EnvironmentRuntimeError>()
    );
    assert!(
        plugin
            .source()
            .unwrap()
            .is::<EnvironmentConfigurationError>()
    );
    for (error, message) in [
        (environment, "docker unavailable"),
        (plugin, "package update failed"),
    ] {
        assert!(matches!(error, ApplicationError::EnvironmentFailed { .. }));
        assert!(format!("{:#}", anyhow::Error::new(error)).contains(message));
    }
}
