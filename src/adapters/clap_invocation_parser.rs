use std::ffi::OsString;

use clap::{Parser, Subcommand};

use crate::{
    contracts::{InvocationParseError, InvocationParser},
    model::{CommandInvocation, CommandOperation},
};

pub struct ClapInvocationParser;

impl InvocationParser for ClapInvocationParser {
    fn parse(&self) -> Result<CommandInvocation, InvocationParseError> {
        parse_from(std::env::args_os())
    }
}

fn parse_from(
    arguments: impl IntoIterator<Item = impl Into<OsString> + Clone>,
) -> Result<CommandInvocation, InvocationParseError> {
    let arguments = ClapArguments::try_parse_from(arguments).map_err(|source| {
        InvocationParseError::ParseFailed {
            source: source.into(),
        }
    })?;

    let operation = match arguments.command {
        None => CommandOperation::Attach,
        Some(ClapCommand::Update { package_names }) => CommandOperation::Update { package_names },
        Some(ClapCommand::Stop) => CommandOperation::Stop,
        Some(ClapCommand::Recreate {
            allow_missing_integrations,
        }) => CommandOperation::Recreate {
            allow_missing_integrations,
        },
        Some(ClapCommand::Remove) => CommandOperation::Remove,
    };

    Ok(CommandInvocation {
        operation,
        additional_package_names: arguments.additional_package_names,
        requested_shell: arguments.requested_shell,
        wayland: arguments.wayland,
        pipewire: arguments.pipewire,
        gpu: arguments.gpu,
        save_settings: arguments.save_settings,
        allow_missing_integrations: arguments.allow_missing_integrations,
    })
}

#[derive(Debug, Parser)]
#[command(
    name = "spawnbx",
    version,
    about = "A containerized development environment powered by Docker and Arch Linux"
)]
struct ClapArguments {
    #[command(subcommand)]
    command: Option<ClapCommand>,

    #[arg(
        long = "shell",
        value_name = "SHELL",
        help = "Shell to start when attaching to the container"
    )]
    requested_shell: Option<String>,

    #[arg(
        long = "packages",
        value_delimiter = ',',
        value_name = "PACKAGES",
        help = "Packages to add to the development container"
    )]
    additional_package_names: Vec<String>,

    #[arg(long = "wayland", help = "Enable wayland support")]
    wayland: bool,

    #[arg(long = "pipewire", help = "Enable pipewire passthrough")]
    pipewire: bool,

    #[arg(long = "gpu", help = "Enable GPU passthrough")]
    gpu: bool,

    #[arg(
        long = "save",
        help = "Save these options to the project settings file"
    )]
    save_settings: bool,

    #[arg(
        long = "allow-missing-integrations",
        help = "Allow creating a container without the missing integrations"
    )]
    allow_missing_integrations: bool,
}

#[derive(Debug, Subcommand)]
enum ClapCommand {
    #[command(about = "Update one or more packages")]
    Update {
        #[arg(value_name = "PACKAGES", help = "Packages to update")]
        package_names: Vec<String>,
    },
    #[command(about = "Stop and delete the container")]
    Remove,
    #[command(about = "Stop the container")]
    Stop,
    #[command(about = "Recreate the container")]
    Recreate {
        #[arg(
            long = "allow-missing-integrations",
            help = "Allow creating a container without the missing integrations"
        )]
        allow_missing_integrations: bool,
    },
}

#[cfg(test)]
#[path = "../../tests/unit/parser.rs"]
mod tests;
