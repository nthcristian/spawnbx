use std::{fs, process::Command};

use anyhow::{Context, Result, ensure};
use serde_json::Value;
use spawnbx::{
    adapters::{DockerEnvironment, NixConfigurator, UserConfigurator},
    contracts::{EnvironmentConfigurator, EnvironmentRuntime},
    model::{CommandInvocation, CommandOperation, CommandOutput, EnvironmentState},
};
use tempfile::TempDir;

const PROFILE: &str = "/nix/var/nix/profiles/default";

// Field order matters: production Drop stops first; cleanup still runs if it panics.
struct RuntimeEnvironment {
    environment: DockerEnvironment,
    cleanup: ContainerCleanup,
    state: EnvironmentState,
}

struct ContainerCleanup {
    name: String,
    workspace: TempDir,
    owner: String,
    armed: bool,
}

impl ContainerCleanup {
    fn remove(&mut self) -> Result<()> {
        if !container_names()?.lines().any(|name| name == self.name) {
            self.armed = false;
            return Ok(());
        }

        // Nix may leave root-owned cache files in the disposable bind-mounted home.
        // Restart after production Drop so even a failed test can release those files.
        let ownership = docker(&["container", "start", &self.name]).and_then(|_| {
            docker(&[
                "exec",
                &self.name,
                "chown",
                "-R",
                &self.owner,
                "/home/spawnbx",
            ])
        });
        let removal = docker(&["container", "rm", "--force", &self.name]);
        if removal.is_ok() {
            self.armed = false;
        }
        removal?;
        ownership.context("container removed, but temporary home ownership cleanup failed")?;
        Ok(())
    }
}

impl Drop for ContainerCleanup {
    fn drop(&mut self) {
        if self.armed
            && let Err(error) = self.remove()
        {
            eprintln!(
                "runtime cleanup failed for test-owned container {} and workspace {}: {error:#}; \
                     inspect these exact resources and remove them manually",
                self.name,
                self.workspace.path().display()
            );
        }
    }
}

impl RuntimeEnvironment {
    fn new() -> Result<Self> {
        ensure!(
            cfg!(debug_assertions),
            "runtime tests require the debug build and local spawnbx:latest image; omit --release"
        );
        docker(&["info"])
            .context("start a reachable local Docker daemon and grant Docker access")?;
        docker(&["image", "inspect", "spawnbx:latest"]).context(
            "build the debug image first: docker build --platform linux/amd64 -t spawnbx:latest .; \
             runtime tests never build or pull images",
        )?;
        let identity = |flag| -> Result<String> {
            let output = Command::new("id").arg(flag).output()?;
            ensure!(output.status.success(), "host id {flag} failed");
            Ok(String::from_utf8(output.stdout)?.trim().to_owned())
        };
        let uid: u32 = identity("-u")?.parse()?;
        let gid: u32 = identity("-g")?.parse()?;
        ensure!(
            uid != 0 && gid != 0,
            "run runtime tests as a non-root host user with nonzero UID/GID"
        );
        let workspace = tempfile::Builder::new()
            .prefix("spawnbx-runtime-")
            .tempdir()?;
        let state = EnvironmentState {
            container_name_prefix: format!(
                "spawnbx-runtime-{}-{}",
                std::process::id(),
                workspace.path().file_name().unwrap().to_string_lossy()
            ),
            workspace_root: workspace
                .path()
                .to_str()
                .context("temporary path is not UTF-8")?
                .into(),
            package_names: Vec::new(),
            wayland: false,
            pipewire: false,
            gpu: false,
            shell_program: "/bin/bash".into(),
            host_username: identity("-un")?,
            host_uid: uid,
            host_gid: gid,
        };
        let environment = DockerEnvironment::try_from(&state)?;
        let name = environment.container_name().to_owned();
        let absent = container_names().and_then(|names| {
            ensure!(
                !names.lines().any(|existing| existing == name),
                "refusing to own or clean up pre-existing container {name}"
            );
            Ok(())
        });
        if let Err(error) = absent {
            // Construction has no runtime resources, but Drop would stop an unowned container.
            std::mem::forget(environment);
            return Err(error);
        }
        Ok(Self {
            environment,
            cleanup: ContainerCleanup {
                name,
                workspace,
                owner: format!("{uid}:{gid}"),
                armed: true,
            },
            state,
        })
    }

    fn execute(&self, args: &[&str]) -> Result<String> {
        checked(
            (self.environment.executor())(args)?,
            &format!("container command {args:?}"),
        )
    }

    fn as_user(&self, args: &[&str]) -> Result<String> {
        let mut command = vec![
            "exec",
            "--user",
            &self.cleanup.owner,
            self.environment.container_name(),
        ];
        command.extend_from_slice(args);
        docker(&command)
    }

    fn configure_user(&self) -> Result<()> {
        UserConfigurator::configure(&self.environment.executor(), &self.state, &invocation())
            .context("real user setup failed; existing user/group script defects are not skipped")
    }

    fn profile(&self) -> Result<Value> {
        Ok(serde_json::from_str(&self.execute(&[
            "nix",
            "profile",
            "list",
            "--json",
            "--profile",
            PROFILE,
        ])?)?)
    }

    fn finish(self) -> Result<()> {
        let Self {
            environment,
            mut cleanup,
            ..
        } = self;
        drop(environment);
        cleanup.remove().with_context(|| {
            format!(
                "clean up test-owned container {} and workspace {}",
                cleanup.name,
                cleanup.workspace.path().display()
            )
        })?;
        // Report filesystem errors that TempDir's destructor would otherwise ignore.
        fs::remove_dir_all(cleanup.workspace.path()).with_context(|| {
            format!(
                "remove disposable runtime workspace {}",
                cleanup.workspace.path().display()
            )
        })
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

fn checked(output: CommandOutput, operation: &str) -> Result<String> {
    ensure!(
        output.exit_code == Some(0),
        "{operation} failed (status {:?}): {}",
        output.exit_code,
        output.stderr
    );
    Ok(output.stdout)
}

fn docker(args: &[&str]) -> Result<String> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .with_context(|| format!("launch docker {args:?}; install Docker CLI and check PATH"))?;
    checked(
        CommandOutput {
            stdout: String::from_utf8(output.stdout)?,
            stderr: String::from_utf8(output.stderr)?,
            exit_code: output.status.code(),
        },
        &format!("docker {args:?}"),
    )
}

fn container_names() -> Result<String> {
    docker(&["container", "ls", "--all", "--format", "{{.Names}}"])
}

#[test]
#[ignore = "requires local Docker, an existing spawnbx:latest debug image, and a non-root host user"]
fn creates_reuses_restarts_and_removes_a_disposable_environment() -> Result<()> {
    let runtime = RuntimeEnvironment::new()?;
    runtime.environment.ensure_running()?;
    let name = runtime.environment.container_name();
    let inspect = || {
        docker(&[
            "container",
            "inspect",
            "--format",
            "{{.Id}} {{.State.Running}}",
            name,
        ])
    };
    let created = inspect()?;
    ensure!(
        created.trim().ends_with(" true"),
        "new container is not running: {created}"
    );
    runtime.environment.ensure_running()?;
    ensure!(
        inspect()? == created,
        "ensure_running replaced a running container"
    );
    docker(&["container", "stop", name])?;
    ensure!(
        inspect()?.trim().ends_with(" false"),
        "container did not stop"
    );
    runtime.environment.ensure_running()?;
    ensure!(
        inspect()? == created,
        "restart did not reuse the same container"
    );
    docker(&["container", "stop", name])?;
    runtime.environment.remove()?;
    ensure!(
        !container_names()?.lines().any(|existing| existing == name),
        "remove left the test container behind"
    );
    runtime.finish()
}

#[test]
#[ignore = "requires local Docker, an existing spawnbx:latest debug image, and a non-root host user"]
fn user_setup_repeats_and_preserves_mapped_identity_and_home_ownership() -> Result<()> {
    let runtime = RuntimeEnvironment::new()?;
    runtime.environment.ensure_running()?;
    runtime.configure_user()?;
    let username = &runtime.state.host_username;
    ensure!(runtime.execute(&["id", "-u", username])?.trim() == runtime.state.host_uid.to_string());
    ensure!(runtime.execute(&["id", "-g", username])?.trim() == runtime.state.host_gid.to_string());
    let passwd = runtime.execute(&["getent", "passwd", username])?;
    let mapped = runtime.as_user(&[
        "sh",
        "-eu",
        "-c",
        "id -u; id -g; pwd; test \"$HOME\" = /home/spawnbx; printf owned > \"$HOME/runtime-owned\"",
    ])?;
    ensure!(
        mapped
            == format!(
                "{}\n{}\n/workspace\n",
                runtime.state.host_uid, runtime.state.host_gid
            )
    );
    runtime.configure_user()?;
    ensure!(
        runtime.execute(&["getent", "passwd", username])? == passwd,
        "repeat setup changed the account"
    );
    let ownership = runtime.execute(&[
        "stat",
        "-c",
        "%u:%g",
        "/home/spawnbx",
        "/home/spawnbx/runtime-owned",
    ])?;
    ensure!(
        ownership == format!("{0}\n{0}\n", runtime.cleanup.owner),
        "home ownership mismatch: {ownership}"
    );
    ensure!(
        fs::read_to_string(
            runtime
                .cleanup
                .workspace
                .path()
                .join(".spawnbx/home/runtime-owned")
        )? == "owned"
    );
    runtime.finish()
}

#[test]
#[ignore = "requires local Docker, spawnbx:latest debug image, non-root user, and Nix registry/network or cache"]
fn installs_hello_and_repeat_configuration_preserves_the_nix_profile() -> Result<()> {
    let mut runtime = RuntimeEnvironment::new()?;
    runtime.environment.ensure_running()?;
    runtime.configure_user()?;
    let hello_count = |profile: &Value| -> Result<usize> {
        let elements = profile["elements"]
            .as_object()
            .context("Nix profile has no elements object")?;
        Ok(elements
            .values()
            .filter(|element| {
                element["active"].as_bool().unwrap_or(true)
                    && element["attrPath"]
                        .as_str()
                        .is_some_and(|attr| attr == "hello" || attr.ends_with(".hello"))
            })
            .count())
    };
    ensure!(
        hello_count(&runtime.profile()?)? == 0,
        "use a baseline debug image without hello already installed"
    );
    runtime.state.package_names = vec!["hello".into()];
    let configure = || {
        NixConfigurator::configure(&runtime.environment.executor(), &runtime.state, &invocation())
        .context("real Nix configuration failed; check registry/network/cache and debug image prerequisites")
    };
    configure()?;
    let installed = runtime.profile()?;
    ensure!(
        hello_count(&installed)? == 1,
        "hello was not installed exactly once: {installed}"
    );
    let generation = runtime.execute(&["readlink", "-f", PROFILE])?;
    configure()?;
    ensure!(
        runtime.profile()? == installed,
        "repeat configuration changed the Nix profile"
    );
    ensure!(
        runtime.execute(&["readlink", "-f", PROFILE])? == generation,
        "repeat configuration created a new generation"
    );
    let hello = runtime.as_user(&[
        "env",
        "LC_ALL=C",
        "sh",
        "-eu",
        "-c",
        ". /etc/profile.d/spawnbx-nix.sh; command -v hello; hello",
    ])?;
    ensure!(
        hello == format!("{PROFILE}/bin/hello\nHello, world!\n"),
        "installed hello was not available to the mapped user: {hello:?}"
    );
    runtime.finish()
}
