# spawnbx

A containerized development environment powered by Podman and Arch Linux.

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
- **SSH access** — OpenSSH server runs inside the container, reachable on port `2222` (user `dev`, password `dev`)

## Requirements

- [Podman](https://podman.io/)
- For NVIDIA GPU passthrough: [nvidia-container-toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html)

## Quick Start

1. **Build the container image:**

    ```sh
    ./build_images
    ```

    This builds the `dev-arch` image from `images/dev-arch/Containerfile`.

2. **Launch the development environment:**

    ```sh
    ./bin/spawnbx
    ```

    Or add `bin/` to your `PATH` for convenience:

    ```sh
    export PATH="$PWD/bin:$PATH"
    spawnbx
    ```

## Usage

```
spawnbx [OPTIONS]

Options:
  -i, --image IMAGE   Container image to use (default: dev-arch)
  -s, --shell SHELL   Shell to set as the SHELL environment variable inside the
                      container (default: /bin/bash)
  -h, --help          Show help message
```

The container is named after the current directory. If a container with that name already exists, `spawnbx` will reattach to it (starting or unpausing if necessary).

### SSH Access

The container runs an OpenSSH server and maps port `2222` on the host to port `22` inside the container. You can connect via:

```sh
ssh -p 2222 dev@localhost
```

The default password is `dev`. SSH is configured with `PasswordAuthentication` enabled and `PermitUserEnvironment` enabled, so any `SSH_*` environment variables set on the host are available inside the container.

## Project Structure

```
spawnbx/
├── bin/
│   └── spawnbx          # Main launcher script
├── images/
│   └── dev-arch/
│       ├── Containerfile   # Arch Linux container image definition
│       └── entrypoint.sh   # Container entrypoint (fixes permissions, starts sshd, launches shell)
├── build_images            # Script to build all container images
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

Run `./build_images` to build it, then launch with:

```sh
spawnbx -i my-image
```
