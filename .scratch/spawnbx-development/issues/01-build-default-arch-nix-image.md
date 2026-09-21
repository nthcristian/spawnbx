# 01: Build the Default Arch/Nix Image

**What to build:** Create the reproducible `linux/amd64` default image that
the Rust CLI will consume. It must provide the stable runtime contract needed
by every later ticket: Arch Linux, Bash, Nix, baseline development tools, and
runtime identity setup. This ticket is complete when the image can be pulled
or built in the development workflow and used independently of spawnbx.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] The image is based on a pinned Arch Linux base digest rather than a
      mutable latest reference.
- [ ] The image targets `linux/amd64` and fails clearly when used on an
      unsupported platform instead of silently relying on emulation.
- [ ] Bash and the baseline development tools are available without project
      configuration.
- [ ] Nix is installed at a pinned version with the profile/store behavior
      required by the package Adapter.
- [ ] The image contains the runtime identity bootstrap needed to run the
      long-lived workload and attached shell as supplied host numeric UID/GID.
- [ ] The image contains no project-specific packages and does not require a
      project Dockerfile or custom image configuration.
- [ ] The image contract records the release version, base digest, Nix
      version, platform, and resulting image digest.
- [ ] A smoke check proves that Bash starts, Nix responds, and a supplied UID
      and GID are reflected in the running process.
- [ ] Image build or pull failures preserve enough output to diagnose the
      missing tool, network failure, or invalid image reference.
