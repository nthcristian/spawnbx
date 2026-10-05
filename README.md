# spawnbx

[![npm](https://img.shields.io/npm/v/spawnbx?logo=npm)](https://www.npmjs.com/package/spawnbx)
[![GHCR](https://img.shields.io/badge/ghcr.io%2Fnthcristian%2Fspawnbx-latest-2496ed?logo=github)](https://github.com/nthcristian/spawnbx/pkgs/container/spawnbx)

`spawnbx` runs a project-local Docker development environment with a Nix system profile. It bind-mounts the project at `/workspace`, creates a container user with the host UID/GID, and stores that user's home at `.spawnbx/home`.

## Production Release

Version 1.0.8 is the first production release. It supports project-local Docker environments, Nix package reconciliation, and optional Wayland, PipeWire, and GPU passthrough.

Integrations apply only when creating a container, so use `spawnbx recreate` after changing them. Missing host prerequisites fail before Docker creation by default. Set `allow_missing_integrations: true` in `.spawnbx.yml` to omit unavailable requested integrations instead.

## Requirements

- Linux x64
- Docker CLI and daemon
- A `.spawnbx.yml` file in the project directory

Install the published CLI with:

```sh
npm install --global spawnbx
```

Build from source with `cargo build --release`.

## Configure

`.spawnbx.yml` is required. Its fields are optional:

```yaml
name: my-project
shell: fish
packages:
  - fish
  - ripgrep
wayland: true
pipewire: true
gpu: true
allow_missing_integrations: true
```

`name` prefixes the Docker container name. `shell` defaults to `bash`; `packages` defaults to an empty list. Packages are installed with Nix into `/nix/var/nix/profiles/default`. Integration booleans and `allow_missing_integrations` default to `false`.

Wayland requires `XDG_RUNTIME_DIR`, `WAYLAND_DISPLAY`, and its host socket. PipeWire requires `XDG_RUNTIME_DIR/pipewire-0`. Xwayland is forwarded only when the host `DISPLAY` points to an available X11 socket; an existing `$XAUTHORITY` or `$HOME/.Xauthority` is mounted when available. GPU passthrough uses `/dev/dri` for AMD/DRM and Docker GPU allocation when NVIDIA devices are present.

## Commands

Run commands from the directory containing `.spawnbx.yml`.

| Command                        | Current behavior                                                                                                                                                            |
| ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `spawnbx`                      | Creates or starts the environment, configures the user and Nix packages, then attaches the configured shell.                                                                |
| `spawnbx update [packages...]` | Configures the environment, requests Nix upgrades for all installed packages or the named installed packages, reconciles configured packages, then exits without attaching. |
| `spawnbx stop`                 | Stops the derived project container and exits.                                                                                                                              |
| `spawnbx remove`               | Removes the derived project container; `.spawnbx/home` remains.                                                                                                             |
| `spawnbx recreate`             | Removes the container, creates or starts it again, configures it, then attaches.                                                                                            |

Example usage:

```sh
spawnbx --shell fish --packages fish,ripgrep
spawnbx --save --shell fish --packages fish,ripgrep
spawnbx --wayland --pipewire --gpu recreate
```

`--shell` overrides the attached shell for the invocation. `--packages` adds comma-separated package names to the project configuration for the invocation. `--wayland`, `--pipewire`, and `--gpu` enable their corresponding container-creation options. `--save` writes the merged values back to `.spawnbx.yml` before Docker work begins. Place these options before the subcommand.

Progress logs use `RUST_LOG`; normal runs default to INFO and `RUST_LOG=off` suppresses them.
