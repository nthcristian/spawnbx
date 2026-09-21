# 02: Implement the Minimal spawnbx Reconciliation Path

**What to build:** Make the bare `spawnbx` command create a useful default
development workload from the current project directory. Establish the deep
`reconcile::run` Interface and the internal Adapter Seams while implementing
only the baseline path: published image, project mount, persistent home,
host identity, bridge networking, and Bash attachment.

**Blocked by:** 01: Build the Default Arch/Nix Image.

**Status:** ready-for-agent

- [ ] The CLI accepts a bare invocation and returns a useful non-zero error
      when Docker or the pinned default image is unavailable.
- [ ] The composition root creates production Adapters and calls one deep
      reconciliation Interface; it does not sequence Docker operations itself.
- [ ] The reconciliation Interface accepts user intent and returns an
      outcome or domain error without exposing Docker handles or raw commands.
- [ ] The current directory is used as the initial project root when no
      project configuration exists.
- [ ] The project is mounted read-write at `/workspace`.
- [ ] A tool-managed home directory under the project is mounted as the
      container user's home and is created safely when absent.
- [ ] The workload process uses the host numeric UID and GID, or fails with a
      clear identity diagnostic when the active Docker mode cannot satisfy it.
- [ ] Bridge networking is used and host networking is not available through
      this path.
- [ ] Bash is attached only after the workload is running; the long-lived
      workload process is not owned by the interactive shell.
- [ ] A second invocation can find and reuse the baseline workload without
      creating an uncontrolled duplicate.
- [ ] Docker and process invocation use literal argument values, never shell
      command strings.
- [ ] Fake Workspace, Host, Workload, Package, and Terminal Adapters allow
      tests to verify the create, reuse, and failure paths without Docker.
