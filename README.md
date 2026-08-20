# spawnbx

A containerized development environment powered by Podman and Arch Linux.

[![npm version](https://img.shields.io/npm/v/spawnbx)](https://www.npmjs.com/package/spawnbx)

## Overview

`spawnbx` provides a ready-to-use, isolated development environment inside a Podman container. It automatically passes through GPU, audio, and GUI capabilities from the host, making it suitable for development that requires hardware acceleration, sound, or graphical applications — all while keeping your host system clean.

## Features

- **Arch Linux base** — up-to-date toolchain with `base-devel`, `git`, `cmake`, `fish`, `zsh`, and more
- **GPU passthrough** — automatic detection of NVIDIA or Intel/AMD GPUs via DRI
- **Audio support** — PipeWire, PulseAudio, and ALSA passthrough from the host
- **GUI support** — Wayland (preferred) or X11 fallback with automatic configuration
- **Persistent home** — `/home/dev` is stored in a named Podman volume, surviving container rebuilds
- **Workspace mounting** — current working directory is mounted at `/workspace`
- **User namespace mapping** — `--userns=keep-id` ensures file ownership matches the host user
- **SSH-ready containers** — containers are placed on the `devcontainers` network and labeled for optional daemon-based SSH routing

## Requirements

- [Podman](https://podman.io/)
- [Node.js](https://nodejs.org/) ≥ 18 (for installing via npm)
- For NVIDIA GPU passthrough: [nvidia-container-toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html)

## Installation

```sh
npm install -g spawnbx
```

This makes both `spawnbx` and `spawnbx-build` available on your `PATH`.

## Quick Start

1. **Build the container image:**

   ```sh
   spawnbx-build
   ```

   This finds every `images/*/Containerfile` and builds it with `podman build`. By default that means the `dev-arch` image.

2. **Launch the development environment:**

   ```sh
   spawnbx
   ```

## Optional SSH Routing Daemon

The SSH routing daemon is not installed or started by the npm package. It is
an optional component that currently requires manual configuration and setup.

The daemon watches running containers with the `DEV_CONTAINER` label and
creates Unix sockets under `${XDG_RUNTIME_DIR}/container-ssh`. To run it:

```sh
cd daemon
systemctl --user enable --now podman.socket
podman network exists devcontainers || podman network create devcontainers
podman compose up --build
```

The containers started by `spawnbx` already use the `devcontainers` network and
the `DEV_CONTAINER` label. The `daemon/` directory is for manual development
and deployment; it is intentionally not included in the npm package.

## Usage

### `spawnbx`

```
spawnbx [OPTIONS]

Options:
  -i, --image IMAGE   Container image to use (default: dev-arch)
  -s, --shell SHELL   Shell to set as the SHELL environment variable inside the
                      container (default: /bin/bash)
  -h, --help          Show help message
```

The container is named after the current directory. If a container with that name already exists, `spawnbx` will reattach to it (starting or unpausing if necessary).

### `spawnbx-build`

```
spawnbx-build
```

Iterates over every subdirectory of `images/` that contains a `Containerfile` and runs `podman build -t <name> .` inside it. The image tag is the directory name (e.g. `dev-arch`).

### SSH Access

The container runs an OpenSSH server. When the optional routing daemon is
running, configure your SSH client to connect through the Unix socket created
for the target container. SSH is configured with `PasswordAuthentication`
enabled and `PermitUserEnvironment` enabled, so any `SSH_*` environment
variables set on the host are available inside the container.

## Project Structure

```
spawnbx/
├── bin/
│   ├── spawnbx          # Main launcher script
│   └── spawnbx-build    # Image builder script
├── images/
│   └── dev-arch/
│       ├── Containerfile   # Arch Linux container image definition
│       └── entrypoint.sh   # Container entrypoint (fixes permissions, starts sshd, launches shell)
├── daemon/                  # Optional manually configured SSH routing daemon
├── package.json            # npm package metadata
├── LICENSE
└── README.md
```

## Custom Images

To add a custom development image, create a new directory under `images/` with a `Containerfile` inside:

```
images/
└── my-image/
    └── Containerfile
```

Run `spawnbx-build` to build it, then launch with:

```sh
spawnbx -i my-image
```
