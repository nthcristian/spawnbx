# spawnbx

`spawnbx` creates and manages a project development container backed by Docker and Nix. It mounts the current project, matches the host user's numeric UID/GID, persists the container user's home under `.spawnbx/`, and attaches an interactive shell.

The current MVP supports Linux `amd64` hosts and Docker.

## Requirements

- Linux `x86_64` host
- Docker installed and running
- Access to the published image at `ghcr.io/nthcristian/spawnbx:latest`

The first run pulls the image. The image contains Arch Linux, Bash, common development tools, Nix, and the runtime setup used by `spawnbx`.

## Install

Install the published CLI from npm:

```sh
npm install --global spawnbx
```

To build from source:

```sh
cargo build --release
./target/release/spawnbx doctor
```

## Basic Use

Run `spawnbx` from a project directory to reconcile its container and attach a shell:

```sh
cd path/to/project
spawnbx
```

The container stops when the attached shell exits. Project files stay on the host, and `.spawnbx/home` preserves container-user state between runs.

Useful commands:

```sh
spawnbx doctor              # Check host and image prerequisites
spawnbx update              # Update the shared Nixpkgs lock input
spawnbx update ripgrep      # Update and identify a package in the result
spawnbx stop                # Stop the project container
spawnbx remove              # Remove it while preserving project state
```

Common options:

```sh
spawnbx --shell fish
spawnbx --packages fish,ripgrep
spawnbx --x11 --wayland --pipewire --gpu
spawnbx --network none
spawnbx --save --shell fish --packages fish,ripgrep
```

Use `--allow-missing-integrations` when an optional desktop or GPU integration should become a warning instead of stopping the run.

## Project Configuration

Configuration is optional. Create `.spawnbx.yml` in the project root:

```yaml
name: my-project
shell: fish
packages:
  - fish
  - ripgrep
x11: false
wayland: false
pipewire: false
gpu: false
network: bridge
```

Command-line options override the configuration for the current invocation. Add `--save` to write the merged values back to `.spawnbx.yml`.

Generated state is kept under `.spawnbx/` and should not be committed. The default image can be overridden for local testing:

```sh
SPAWNBX_IMAGE=ghcr.io/nthcristian/spawnbx:latest spawnbx doctor
```

## Limitations

- The MVP supports Docker and Linux `amd64` only.
- Project packages are installed with Nix, not pacman.
- X11, Wayland, PipeWire, and GPU access are opt-in.
- The project directory is trusted code and is mounted into the workload.

## License

spawnbx is distributed under the GNU General Public License v3.
