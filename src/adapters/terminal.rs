use crate::reconcile::{
    AdapterError, AttachmentOutcome, ProcessAdapter, ProcessCommand, ShellName, TerminalAdapter,
    WorkloadHandle,
};

pub(crate) struct OsTerminalAdapter {
    process: Box<dyn ProcessAdapter>,
}

pub(crate) struct FakeTerminalAdapter;

impl OsTerminalAdapter {
    pub(crate) fn new(process: Box<dyn ProcessAdapter>) -> Self {
        Self { process }
    }
}

impl FakeTerminalAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl TerminalAdapter for OsTerminalAdapter {
    fn attach(
        &mut self,
        target: &WorkloadHandle,
        shell: &ShellName,
    ) -> Result<AttachmentOutcome, AdapterError> {
        eprintln!(
            "spawnbx: attaching {} shell as uid {} gid {}",
            shell.executable, target.identity.uid, target.identity.gid
        );
        let status = self.process.attach(ProcessCommand {
            executable: "docker".into(),
            arguments: vec![
                "exec".into(),
                "--user".into(),
                format!("{}:{}", target.identity.uid, target.identity.gid).into(),
                "-it".into(),
                target.opaque_id.clone().into(),
                shell.executable.clone().into(),
            ],
        })?;
        eprintln!("spawnbx: attach process exited with {status}");
        Ok(AttachmentOutcome::Interactive(status))
    }
}

impl TerminalAdapter for FakeTerminalAdapter {
    fn attach(
        &mut self,
        _target: &WorkloadHandle,
        _shell: &ShellName,
    ) -> Result<AttachmentOutcome, AdapterError> {
        Ok(AttachmentOutcome::Skipped)
    }
}
