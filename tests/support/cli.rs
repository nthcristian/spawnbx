use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output},
};

pub struct Cli {
    _root: tempfile::TempDir,
    pub workspace: PathBuf,
    pub bin: PathBuf,
    pub records: PathBuf,
}

impl Cli {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace with spaces");
        let bin = root.path().join("bin");
        let records = root.path().join("records");
        for path in [&workspace, &bin, &records] {
            fs::create_dir(path).unwrap();
        }
        for (name, contents) in [
            ("docker", include_str!("../fixtures/docker.sh")),
            ("id", include_str!("../fixtures/id.sh")),
        ] {
            let path = bin.join(name);
            fs::write(&path, contents).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let cli = Self {
            _root: root,
            workspace,
            bin,
            records,
        };
        cli.settings("name: character\nshell: fish\npackages: []\n");
        cli
    }

    pub fn settings(&self, yaml: &str) {
        fs::write(self.workspace.join(".spawnbx.yml"), yaml).unwrap();
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_spawnbx"));
        command
            .current_dir(&self.workspace)
            .env_clear()
            .env("PATH", &self.bin)
            .env("FAKE_ROOT", &self.records)
            .env("FAKE_PROFILE", r#"{"elements":{}}"#)
            .env("RUST_LOG", "off");
        command
    }

    pub fn run(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        self.command()
            .args(args)
            .envs(env.iter().copied())
            .output()
            .unwrap()
    }

    pub fn name(&self) -> String {
        // Independent copy of the persisted container-identity baseline.
        let mut hasher = DefaultHasher::new();
        self.workspace.to_str().unwrap().hash(&mut hasher);
        let suffix: String = hasher.finish().to_be_bytes()[..4]
            .iter()
            .map(|byte| format!("{:02}", byte % 99 + 1))
            .collect();
        format!("character-{suffix}")
    }

    pub fn calls(&self, executable: &str) -> Vec<Vec<String>> {
        let bytes = fs::read(self.records.join(format!("{executable}.args"))).unwrap_or_default();
        let mut calls = vec![];
        let mut call = vec![];
        for arg in bytes.split(|byte| *byte == 0) {
            if arg.is_empty() {
                if !call.is_empty() {
                    calls.push(std::mem::take(&mut call));
                }
            } else {
                call.push(String::from_utf8(arg.to_vec()).unwrap());
            }
        }
        calls
    }

    pub fn events(&self) -> Vec<String> {
        fs::read_to_string(self.records.join("events"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    pub fn phases(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter(|line| !line.starts_with("id:"))
            .map(|line| line.split(':').next().unwrap().to_owned())
            .collect()
    }
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub fn assert_success(output: &Output) {
    assert!(output.status.success(), "{}", stderr(output));
}

pub fn assert_failure(output: &Output, message: &str) {
    assert!(!output.status.success());
    assert!(stderr(output).contains(message), "{}", stderr(output));
}
