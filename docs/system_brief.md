# System Brief

Spawnbx creates a Docker development environment for the current project and configures it with a host-mapped user and a Nix system profile.

The project configuration controls the container-name prefix, attached shell, and package list. The project directory and `.spawnbx/home` are mounted into the container.

It does not provide desktop, GPU, network, or host-integration controls. It also does not create Nix lock files or persist a project-local Nix profile.
