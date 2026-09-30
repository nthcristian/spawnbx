# Current Requirements

- Linux x64, Docker, and a project-local `.spawnbx.yml` are required.
- The settings file supports `name`, `shell`, and `packages`; `--save` writes those same fields.
- The container mounts the project at `/workspace` and the container home at `.spawnbx/home`.
- The container user is configured with the host username, UID, and GID.
- Packages are reconciled with Nix in `/nix/var/nix/profiles/default`.

Desktop integrations, networking controls, lock-file reproducibility, and project-local Nix profile persistence are not implemented.
