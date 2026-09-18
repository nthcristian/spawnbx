# spawnbx Architectural Design

## Status

Approved through collaborative design review on 2026-09-18. This document
defines the MVP architecture and intentionally does not define an
implementation plan.

## Summary

`spawnbx` is a Linux-first Rust CLI for creating and reusing development
containers without requiring project configuration. Its default behavior is
to reconcile the current project with a published, digest-pinned Arch Linux
image, install project packages through Nix, and attach an interactive Bash
shell.

Project files remain on the host. The project directory is mounted into the
container, while a tool-managed home directory under the project preserves
Nix state, the user profile, shell history, and other container-user state
across container recreation.

The MVP targets `linux/amd64`, Docker, and trusted developer workspaces. It
supports opt-in X11, Wayland, PipeWire, and GPU integration, with explicit
preflight checks and no privileged fallback.

## Goals

- Start a useful development container from a command such as
  `spawnbx --shell fish --x11`.
- Require no project configuration for the default workflow.
- Support optional project configuration in `.spawnbx.yml`.
- Reuse a matching container and recreate it when its desired configuration
  or image changes.
- Keep project files and ownership on the host.
- Match the host user's numeric UID and GID in the container.
- Install project packages with Nix and preserve reproducibility with a
  project-local lockfile.
- Provide explicit, least-privilege desktop and GPU integrations.
- Produce actionable diagnostics before mutating Docker when a requested
  integration cannot be provided.

## Non-Goals

The MVP does not include custom images or user Dockerfiles, local image builds,
ARM support, macOS or Windows host integration, host networking, arbitrary
Docker network modes, arbitrary Nix flakes or overlays, pacman-managed project
packages, CUDA/ROCm/compute GPU features, broad host socket or device mounts,
or isolation from malicious project code.

## User Workflow

### Project discovery

Starting at the current directory, `spawnbx` searches ancestors for the
nearest `.spawnbx.yml`. If no file is found, the current directory is the
project root. The resolved root is reported in diagnostics and is the only
root under which project-specific state may be read or written.

All state paths are canonicalized and checked so symlinks cannot redirect
tool-managed state outside the project root. An explicit project-directory
override may be added later; it is not part of the MVP contract.

### Configuration and overrides

`.spawnbx.yml` is the only user-authored project configuration. Its initial
shape is:

```yaml
name: my-editor
shell: fish
packages:
  - fish
  - ripgrep
  - python3Packages.black
x11: true
wayland: true
pipewire: true
gpu: true
network: bridge
```

Supported fields are:

- `name`: optional readable project name. The absolute-path hash is always
  appended to the container name.
- `shell`: optional executable name. The default is `bash`.
- `packages`: bare nixpkgs attribute paths, such as `ripgrep` or
  `python3Packages.black`.
- `x11`, `wayland`, `pipewire`, and `gpu`: independent opt-in integrations.
- `network`: `bridge` by default or `none`.

The persistent home location is fixed at `.spawnbx/home` in the project and
is not configurable in the MVP.

CLI flags mirror common configuration fields: `--shell`, `--packages`,
`--x11`, `--wayland`, `--pipewire`, `--gpu`, `--network`, `--save`, and
`--allow-missing-integrations`.
Flags override YAML for the current invocation. `--packages` replaces the
configured package list for that invocation. `--save` writes the merged
configuration to `.spawnbx.yml`. A missing configuration is valid and means
the default image, Bash, no extra packages, and no desktop integrations.

### Container naming

The default name is:

```text
<folder-name>-<absolute-path-hash>
```

When `name` is configured, the name is:

```text
<custom-name>-<absolute-path-hash>
```

The hash is derived from the canonical project path rather than the spelling
of the current working directory. It uses the first 12 characters of the
SHA-256 digest encoded as lowercase hex. Folder and custom names are
normalized to Docker-compatible name characters; an empty result is rejected.
A same-named unmanaged Docker container is never modified; reconciliation
fails with a collision diagnostic.

### Command surface

- `spawnbx`: reconcile and attach to the project container.
- `spawnbx update`: advance the shared Nixpkgs lock input and reconcile.
- `spawnbx update <package>`: perform the same shared-input update while
  identifying the requested package in the result. This does not promise an
  isolated package version update; other package resolutions may change.
- `spawnbx stop`: stop the project container without deleting project state.
- `spawnbx remove`: remove the project container while preserving project
  files and `.spawnbx/home`.
- `spawnbx doctor`: run Docker, Nix, identity, socket, device, and image
  preflight without mutating the container.

## Architecture

The CLI uses a deterministic reconciliation loop with narrow process
adapters. Docker and Nix are invoked as executables with literal argument
arrays, never through a shell.

### Components

- **Command layer:** parses subcommands and flags and renders diagnostics.
- **Project/config layer:** discovers the project root, parses YAML, applies
  overrides, and writes explicit `--save` changes.
- **Desired-state layer:** applies defaults, resolves names, and computes the
  canonical desired specification and its hash.
- **Preflight layer:** validates Docker mode, platform, image, sockets,
  devices, groups, and permissions without mutation.
- **Docker adapter:** performs Docker inspection, image operations, container
  lifecycle operations, and shell execution/attachment.
- **Nix adapter:** generates internal Nix metadata, creates and updates the
  lock, and reconciles the persistent profile.
- **Reconciler:** sequences preflight, image availability, container
  lifecycle, Nix setup, and shell attachment.
- **State and diagnostics layer:** owns `.spawnbx`, safe path checks, failure
  categories, labels, and remediation messages.

Each adapter is replaceable in tests. The MVP intentionally delegates Docker
contexts, credentials, transports, and Nix store behavior to their supported
CLIs rather than implementing those protocols directly.

### Reconciliation flow

1. Resolve the project root and load `.spawnbx.yml` if present.
2. Apply CLI overrides and optionally save the merged configuration.
3. Normalize configuration into a desired specification containing the image
   digest, name, identity, shell, packages, mounts, network, and integrations.
4. Run all host preflight checks before creating, removing, or changing a
   container.
5. Ensure the published `linux/amd64` image is available by immutable digest.
6. Inspect the named container's `spawnbx` labels.
7. Reuse a matching running container, start a matching stopped container, or
   replace a mismatched managed container.
8. Reconcile the generated Nix profile inside the container.
9. Attach the requested shell. Bash is the fallback; an explicit shell must
   be installed and executable.

The long-lived container process is separate from the attached interactive
shell, allowing the container to be stopped and restarted without making the
shell the container's lifecycle owner.

## Image Contract

The default image is a separately built and published release artifact. The
CLI references a versioned image by immutable digest and pulls it on first
use. A cached image can be used offline; the MVP does not build images
locally.

The image contract is:

- Arch Linux on `linux/amd64`.
- Bash and baseline development tools.
- A pinned Nix version and configured Nix daemon/profile support.
- A runtime identity bootstrap capable of running the project process as the
  host numeric UID/GID.
- No project-specific packages.

The image digest, Arch base-image provenance, Nix version, and image build
source are release metadata. The project lockfile does not replace image
pinning.

## Nix Package Lifecycle

Arch remains the base operating system, but project packages are installed
with Nix, never with pacman.

`spawnbx` generates deterministic internal Nix metadata under `.spawnbx/`
from the YAML package list. The generated metadata references one `nixpkgs`
input and exposes the configured packages as an aggregate profile target.
Package declarations remain only in `.spawnbx.yml`; users do not need to edit
the generated expression.

The associated `.spawnbx/flake.lock` is the project reproducibility artifact:

- The first package reconciliation creates the metadata and lock, then
  installs the aggregate target into a dedicated profile under
  `.spawnbx/home`.
- A package-list change regenerates the expression and reconciles the profile
  against the existing locked Nixpkgs revision.
- Removing a package removes it from the profile's desired generation. Nix
  store paths may remain until normal garbage collection.
- `spawnbx update` advances the shared Nixpkgs input and reconciles all
  packages.
- `spawnbx update <package>` advances the same shared input and reports the
  requested package's result; it does not isolate that package from other
  package changes.
- Ordinary reconciliation never silently advances the lock.

Nix commands are non-interactive and lock-respecting. Invalid package
attributes, evaluation failures, unavailable inputs, and profile failures
stop reconciliation while preserving the original Nix diagnostics.

## Identity, Mounts, and Persistence

- The host project root is mounted read-write at `/workspace`.
- `.spawnbx/home` is mounted as the container user's home directory.
- The container process uses the host user's numeric UID and GID where the
  active Docker mode permits it.
- Only integration-specific supplementary groups are added.
- No host home directory, Docker socket, or entire runtime directory is
  mounted.

The image's identity bootstrap creates or resolves the runtime user as needed,
then the long-lived process and attached shell run as that user. If rootless
mapping or filesystem permissions cannot satisfy the requested identity, the
operation fails with remediation instead of silently changing ownership
semantics.

## Desktop and GPU Integrations

All integrations are independent opt-ins and are validated before Docker
mutation.

### X11

Pass `DISPLAY`, mount only `/tmp/.X11-unix` read-only, and mount the relevant
Xauthority file read-only when present. Never use `xhost +`.

### Wayland

Pass `WAYLAND_DISPLAY` and mount only the selected Wayland socket. Provide a
container-local `XDG_RUNTIME_DIR`; do not expose the whole host runtime
directory.

### PipeWire

Pass the selected PipeWire remote and mount only its socket. PulseAudio
compatibility sockets and portals are not assumptions of the MVP.

### GPU

NVIDIA requires the NVIDIA Container Toolkit and Docker GPU integration.
AMD/Intel support exposes the available DRM render nodes, typically under
`/dev/dri/renderD*`, because the MVP has no per-GPU selection field. It never
exposes the whole `/dev/dri` directory or KMS devices. CUDA, ROCm, and vendor
compute runtimes are also outside the MVP.

### Security behavior

Rootless Docker is preferred. Rootful Docker is allowed as an automatic
fallback but produces an explicit warning. The container never uses
`--privileged`, host PID/network namespaces, host library-directory mounts,
or automatic forwarding of all host environment variables.

Bridge networking is the default. `network: none` is an explicit restricted
mode and changing the network mode requires recreation. Host networking and
arbitrary Docker network modes are not supported. A package installation in
`network: none` succeeds only when all required Nix inputs and store paths are
already available; otherwise reconciliation fails with an offline-cache
diagnostic rather than temporarily enabling network access.

Requested integration failures are fatal before container mutation. The
explicit `--allow-missing-integrations` override omits unavailable requested
integrations and continues with a warning. This override never broadens
privileges or substitutes an unrelated integration.

The tool is for trusted project code. Display servers, PipeWire, and GPU
devices can expose host-session capabilities and are not a malicious-code
sandbox.

## Container State and Errors

Managed containers carry labels for project identity, desired-spec hash, image
digest, and schema version.

- No container: create it.
- Matching running container: attach after Nix reconciliation.
- Matching stopped container: start it, reconcile Nix, and attach.
- Mismatched managed container: stop, remove, and recreate while preserving
  host-mounted state.
- Same-named unmanaged container: fail without modifying it.
- Failed Nix reconciliation: report the Nix error and retain the managed
  container for diagnosis rather than silently deleting it.

Process adapter errors preserve exit status, stdout, and stderr. Errors are
classified as missing executable, unavailable service, invalid machine
output, non-zero command, or preflight failure. Mutating operations are not
retried automatically.

## Verification Strategy

### Unit tests

- YAML parsing and validation.
- CLI/YAML merge and `--save` behavior.
- Package normalization and generated Nix metadata decisions.
- Project-root discovery and safe path checks.
- Canonical desired-spec hashing and name generation.
- Lock creation/update rules.

### Adapter tests

Fake process runners assert exact Docker and Nix argument construction,
including mounts, labels, identity, device access, and error preservation.

### Reconciliation tests

Cover create, reuse, restart, recreation, image changes, configuration
changes, unmanaged name collisions, missing integrations, and failed package
installation.

### Integration tests

A small Linux suite uses a real Docker daemon, the published image, and a
minimal Nix package set. Desktop and GPU smoke tests run where the relevant
host hardware/session exists; unsupported hardware must produce deterministic
diagnostics rather than being silently treated as success.
