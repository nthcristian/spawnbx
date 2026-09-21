use std::ffi::OsString;

use crate::reconcile::{Invocation, NetworkMode, Operation, Overrides, PackageName};

pub(crate) struct Cli {
    pub(crate) invocation: Invocation,
}

impl Cli {
    pub(crate) fn parse<I>(arguments: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = OsString>,
    {
        let values = arguments
            .into_iter()
            .skip(1)
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let mut index = 0;
        let mut operation = Operation::Attach;

        if let Some(command) = values.first() {
            match command.as_str() {
                "update" => {
                    index += 1;
                    let package = values
                        .get(index)
                        .filter(|value| !value.starts_with('-'))
                        .map(|value| {
                            index += 1;
                            PackageName {
                                attribute_path: value.clone(),
                            }
                        });
                    operation = Operation::Update { package };
                }
                "stop" => {
                    operation = Operation::Stop;
                    index += 1;
                }
                "remove" => {
                    operation = Operation::Remove;
                    index += 1;
                }
                "doctor" => {
                    operation = Operation::Doctor;
                    index += 1;
                }
                value if value.starts_with('-') => {}
                value => return Err(format!("unknown command: {value}")),
            }
        }

        let mut overrides = Overrides::default();
        let mut save = false;
        let mut allow_missing_integrations = false;
        while index < values.len() {
            let flag = values[index].as_str();
            index += 1;
            match flag {
                "--save" => save = true,
                "--allow-missing-integrations" => allow_missing_integrations = true,
                "--x11" => overrides.x11 = Some(true),
                "--wayland" => overrides.wayland = Some(true),
                "--pipewire" => overrides.pipewire = Some(true),
                "--gpu" => overrides.gpu = Some(true),
                "--shell" => overrides.shell = Some(next_value(&values, &mut index, flag)?),
                "--packages" => {
                    overrides.packages = Some(
                        next_value(&values, &mut index, flag)?
                            .split(',')
                            .map(str::trim)
                            .filter(|package| !package.is_empty())
                            .map(str::to_owned)
                            .collect(),
                    );
                }
                "--network" => {
                    overrides.network =
                        Some(match next_value(&values, &mut index, flag)?.as_str() {
                            "bridge" => NetworkMode::Bridge,
                            "none" => NetworkMode::None,
                            other => return Err(format!("invalid network mode: {other}")),
                        });
                }
                "--help" | "-h" => return Err(usage()),
                other => return Err(format!("unknown flag: {other}")),
            }
        }

        Ok(Self {
            invocation: Invocation {
                cwd: std::env::current_dir().map_err(|error| error.to_string())?,
                operation,
                overrides,
                save,
                allow_missing_integrations,
            },
        })
    }
}

fn next_value(values: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    let value = values
        .get(*index)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))?;
    *index += 1;
    Ok(value)
}

fn usage() -> String {
    [
        "usage: spawnbx [update [package]|stop|remove|doctor] [flags]",
        "flags: --shell SHELL --packages PKG1,PKG2 --x11 --wayland --pipewire --gpu",
        "       --network bridge|none --save --allow-missing-integrations",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use crate::reconcile::{NetworkMode, Operation};

    fn arguments(values: &[&str]) -> Vec<std::ffi::OsString> {
        std::iter::once("spawnbx")
            .chain(values.iter().copied())
            .map(std::ffi::OsString::from)
            .collect()
    }

    #[test]
    fn parses_attach_overrides() {
        let cli = Cli::parse(arguments(&[
            "--shell",
            "fish",
            "--packages",
            "fish,ripgrep",
            "--x11",
            "--network",
            "none",
            "--save",
        ]))
        .expect("valid invocation");

        assert!(matches!(cli.invocation.operation, Operation::Attach));
        assert_eq!(cli.invocation.overrides.shell.as_deref(), Some("fish"));
        assert_eq!(
            cli.invocation.overrides.packages,
            Some(vec!["fish".to_owned(), "ripgrep".to_owned()])
        );
        assert_eq!(cli.invocation.overrides.network, Some(NetworkMode::None));
        assert!(cli.invocation.save);
    }

    #[test]
    fn parses_update_focus_without_attaching() {
        let cli = Cli::parse(arguments(&[
            "update",
            "ripgrep",
            "--allow-missing-integrations",
        ]))
        .expect("valid invocation");

        assert!(matches!(
            cli.invocation.operation,
            Operation::Update { package: Some(_) }
        ));
        assert!(cli.invocation.allow_missing_integrations);
    }
}
