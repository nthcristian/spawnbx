# devenv

A containerized development environment powered by Podman and Arch Linux.

## Overview

`devenv` provides a ready-to-use, isolated development environment inside a Podman container. It automatically passes through GPU, audio, and GUI capabilities from the host, making it suitable for development that requires hardware acceleration, sound, or graphical applications — all while keeping your host system clean.

## Features

- **Arch Linux base** — up-to-date toolchain with `base-devel`, `git`, `cmake`, and more
- **GPU passthrough** — automatic detection of NVIDIA or Intel/AMD GPUs via DRI
- **Audio support** — PipeWire, PulseAudio, and ALSA passthrough
- **GUI support** — Wayland (preferred) or X11 fallback with automatic configuration
- **Persistent home** — `/home/dev` is stored in a named Podman volume, surviving container rebuilds
- **Workspace mounting** — current working directory is mounted at `/workspace`
- **User namespace mapping** — `--userns=keep-id` ensures file ownership matches the host user

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
    ./bin/devenv
    ```

    Or add `bin/` to your `PATH` for convenience:

    ```sh
    export PATH="$PWD/bin:$PATH"
    devenv
    ```

## Usage

```
devenv [OPTIONS]

Options:
  -i, --image IMAGE   Container image to use (default: dev-arch)
  -s, --shell SHELL   Shell to use inside the container (default: /bin/bash)
  -h, --help          Show help message
```

The container is named after the current directory. If a container with that name already exists, `devenv` will reattach to it (starting or unpausing if necessary).

## Project Structure

```
devenv/
├── bin/
│   └── devenv          # Main launcher script
├── images/
│   └── dev-arch/
│       ├── Containerfile   # Arch Linux container image definition
│       └── entrypoint.sh   # Container entrypoint
├── build_images            # Script to build all container images
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
devenv -i my-image
```
