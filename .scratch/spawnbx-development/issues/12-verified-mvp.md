# 12: Complete the Verified MVP

**What to build:** Consolidate the implemented vertical slices into a
repeatable development and release workflow. The finished MVP must be
verifiable from a clean Linux `linux/amd64` host, with deterministic behavior
when Docker, Nix, desktop sessions, or GPU prerequisites are missing.

**Blocked by:** 01: Build the Default Arch/Nix Image; 02: Implement the
Minimal spawnbx Reconciliation Path; 03: Add Project Configuration and CLI
Overrides; 04: Add Strict Preflight and Diagnostic Errors; 05: Implement
Managed Desired-State Reconciliation; 06: Implement Nix Profile and Lock
Reconciliation; 07: Add Stop and Remove Lifecycle Commands; 08: Add X11
Forwarding; 09: Add Wayland Forwarding; 10: Add PipeWire Forwarding; 11: Add
NVIDIA and AMD/Intel GPU Forwarding.

**Status:** ready-for-agent

- [ ] Unit and module tests cover configuration, desired state, preflight,
      lifecycle decisions, package plans, and diagnostic classification.
- [ ] Fake-Adapter reconciliation tests cover create, reuse, restart,
      recreation, collision, preflight failure, package failure, and attach
      failure without reaching past the caller-facing Interface.
- [ ] Docker/Nix integration tests exercise the default image, persistent home,
      lock-respecting package installation, package updates, and recreation.
- [ ] Desktop smoke checks are available for X11, Wayland, and PipeWire where
      the host session provides the required resources.
- [ ] GPU smoke checks are available for NVIDIA and AMD/Intel hosts where the
      required runtime or render nodes exist.
- [ ] Unsupported or unavailable host capabilities produce deterministic
      diagnostics and are never silently treated as successful passthrough.
- [ ] The default image reference and digest are validated as part of release
      checks.
- [ ] Documentation covers installation, the default command, YAML fields,
      overrides and save behavior, lock updates, persistent state, security
      assumptions, rootless/rootful behavior, desktop setup, GPU setup, and
      troubleshooting.
- [ ] The complete test and smoke workflow passes without requiring custom
      images, ARM support, host networking, arbitrary Nix flakes, or privileged
      containers.
