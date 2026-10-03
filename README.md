# spawnbx

[![npm](https://img.shields.io/npm/v/spawnbx?logo=npm)](https://www.npmjs.com/package/spawnbx)
[![GHCR](https://img.shields.io/badge/ghcr.io%2Fnthcristian%2Fspawnbx-latest-2496ed?logo=github)](https://github.com/nthcristian/spawnbx/pkgs/container/spawnbx)

`spawnbx` runs a project-local Docker development environment with a Nix system profile. It bind-mounts the project at `/workspace`, creates a container user with the host UID/GID, and stores that user's home at `.spawnbx/home`.

## Status

Wayland, PipeWire, and AMD GPU passthrough are available when creating a container. Wayland and PipeWire require host `XDG_RUNTIME_DIR`; Wayland also requires `WAYLAND_DISPLAY` and `DISPLAY` to pass through Xwayland. An existing `$XAUTHORITY` or `$HOME/.Xauthority` is mounted when available; otherwise the X11 socket is still passed through. AMD GPU passthrough mounts `/dev/dri`. Use `spawnbx recreate` after enabling or changing these options. Network and `--allow-missing-integrations` options remain unavailable.

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
```

`name` prefixes the Docker container name. `shell` defaults to `bash`; `packages` defaults to an empty list. Packages are installed with Nix into `/nix/var/nix/profiles/default`. Integration booleans default to `false`.

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

`--shell` overrides the attached shell for the invocation. `--packages` adds comma-separated package names to the project configuration for the invocation. `--wayland`, `--pipewire`, and `--gpu` enable their corresponding container-creation options. `--save` writes the merged values back to `.spawnbx.yml` before Docker work begins.

Progress logs use `RUST_LOG`; normal runs default to INFO and `RUST_LOG=off` suppresses them.
