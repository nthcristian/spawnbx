use super::*;
use clap::error::ErrorKind;

#[test]
fn no_arguments_defaults_to_attach_without_overrides() {
    let invocation = parse_from(["spawnbx"]).unwrap();
    assert_eq!(invocation.operation, CommandOperation::Attach);
    assert!(invocation.additional_package_names.is_empty());
    assert!(invocation.requested_shell.is_none());
    assert!(!invocation.save_settings);
}

#[test]
fn every_subcommand_preserves_its_operation_and_update_arguments() {
    for (arguments, expected) in [
        (vec!["remove"], CommandOperation::Remove),
        (vec!["stop"], CommandOperation::Stop),
        (vec!["recreate"], CommandOperation::Recreate),
        (
            vec!["update"],
            CommandOperation::Update {
                package_names: vec![],
            },
        ),
        (
            vec!["update", "git", "git", "curl,wget"],
            CommandOperation::Update {
                package_names: vec!["git".into(), "git".into(), "curl,wget".into()],
            },
        ),
    ] {
        let invocation = parse_from(std::iter::once("spawnbx").chain(arguments)).unwrap();
        assert_eq!(invocation.operation, expected);
    }
}

#[test]
fn options_before_subcommand_preserve_shell_packages_duplicates_and_save() {
    let invocation = parse_from([
        "spawnbx",
        "--shell",
        "/custom/shell --flag",
        "--packages",
        "git,curl,git",
        "--packages",
        "wget",
        "--save",
        "update",
        "git",
    ])
    .unwrap();
    assert_eq!(
        invocation.requested_shell.as_deref(),
        Some("/custom/shell --flag")
    );
    assert_eq!(
        invocation.additional_package_names,
        ["git", "curl", "git", "wget"]
    );
    assert!(invocation.save_settings);
    assert_eq!(
        invocation.operation,
        CommandOperation::Update {
            package_names: vec!["git".into()]
        }
    );
}

#[test]
fn delimiter_allows_literal_update_arguments_without_comma_splitting() {
    let invocation = parse_from(["spawnbx", "update", "--", "--save", "one,two"]).unwrap();
    assert_eq!(
        invocation.operation,
        CommandOperation::Update {
            package_names: vec!["--save".into(), "one,two".into()],
        }
    );
    assert!(!invocation.save_settings);
}

#[test]
fn empty_package_segments_are_preserved() {
    let invocation = parse_from(["spawnbx", "--packages=git,,curl,"]).unwrap();
    assert_eq!(invocation.additional_package_names, ["git", "", "curl", ""]);
}

#[test]
fn non_utf8_option_values_are_parse_failures() {
    use std::os::unix::ffi::OsStringExt;

    let arguments = [
        OsString::from("spawnbx"),
        OsString::from("--shell"),
        OsString::from_vec(vec![0xff]),
    ];
    let InvocationParseError::ParseFailed { source } = parse_from(arguments).unwrap_err() else {
        panic!("non-UTF-8 arguments changed error classification");
    };
    assert_eq!(
        source.downcast_ref::<clap::Error>().unwrap().kind(),
        ErrorKind::InvalidUtf8
    );
}

#[test]
fn invalid_arguments_and_post_subcommand_options_remain_parse_failed() {
    for arguments in [
        vec!["spawnbx", "unknown"],
        vec!["spawnbx", "--unknown"],
        vec!["spawnbx", "--shell"],
        vec!["spawnbx", "--packages"],
        vec!["spawnbx", "--save=true"],
        vec!["spawnbx", "stop", "extra"],
        vec!["spawnbx", "update", "--save"],
        vec!["spawnbx", "remove", "--shell", "zsh"],
        vec!["spawnbx", "recreate", "--packages", "git"],
    ] {
        let error = parse_from(arguments).unwrap_err();
        assert!(matches!(error, InvocationParseError::ParseFailed { .. }));
    }
}

#[test]
fn characterization_help_and_version_are_parse_failures_not_successful_exits() {
    for (arguments, kind) in [
        (vec!["spawnbx", "--help"], ErrorKind::DisplayHelp),
        (vec!["spawnbx", "-h"], ErrorKind::DisplayHelp),
        (vec!["spawnbx", "update", "--help"], ErrorKind::DisplayHelp),
        (vec!["spawnbx", "--version"], ErrorKind::DisplayVersion),
        (vec!["spawnbx", "-V"], ErrorKind::DisplayVersion),
    ] {
        let InvocationParseError::ParseFailed { source } = parse_from(arguments).unwrap_err()
        else {
            panic!("help/version changed error classification");
        };
        assert_eq!(source.downcast_ref::<clap::Error>().unwrap().kind(), kind);
    }
}
