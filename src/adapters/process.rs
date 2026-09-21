use std::process::{Command, Stdio};

use crate::reconcile::{AdapterError, ProcessAdapter, ProcessCommand, ProcessOutput};

pub(crate) struct SystemProcessAdapter;

pub(crate) struct RecordingProcessAdapter {
    pub(crate) commands: Vec<ProcessCommand>,
}

impl SystemProcessAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl RecordingProcessAdapter {
    pub(crate) fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl ProcessAdapter for SystemProcessAdapter {
    fn run(&mut self, command: ProcessCommand) -> Result<ProcessOutput, AdapterError> {
        let mut process = Command::new(&command.executable);
        process.args(&command.arguments);
        let output = process.output().map_err(|error| AdapterError {
            category: "process-spawn".to_owned(),
            message: format!("{}: {error}", command_text(&command)),
        })?;

        Ok(ProcessOutput {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    fn attach(
        &mut self,
        command: ProcessCommand,
    ) -> Result<std::process::ExitStatus, AdapterError> {
        let mut process = Command::new(&command.executable);
        process
            .args(&command.arguments)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        process.status().map_err(|error| AdapterError {
            category: "process-spawn".to_owned(),
            message: format!("{}: {error}", command_text(&command)),
        })
    }
}

impl ProcessAdapter for RecordingProcessAdapter {
    fn run(&mut self, command: ProcessCommand) -> Result<ProcessOutput, AdapterError> {
        self.commands.push(command);
        Err(AdapterError {
            category: "recording-process".to_owned(),
            message: "recording adapter has no process output".to_owned(),
        })
    }

    fn attach(
        &mut self,
        command: ProcessCommand,
    ) -> Result<std::process::ExitStatus, AdapterError> {
        self.commands.push(command);
        Err(AdapterError {
            category: "recording-process".to_owned(),
            message: "recording adapter has no process status".to_owned(),
        })
    }
}

fn command_text(command: &ProcessCommand) -> String {
    let mut values = Vec::with_capacity(command.arguments.len() + 1);
    values.push(command.executable.to_string_lossy().into_owned());
    values.extend(
        command
            .arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned()),
    );
    values.join(" ")
}
