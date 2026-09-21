# 11: Add NVIDIA and AMD/Intel GPU Forwarding

**What to build:** Make GPU access a strict, opt-in capability that supports
NVIDIA through the NVIDIA Container Toolkit and AMD/Intel through DRM render
nodes, while preserving the trusted-workspace security constraints.

**Blocked by:** 04: Add Strict Preflight and Diagnostic Errors; 05: Implement
Managed Desired-State Reconciliation.

**Status:** ready-for-agent

- [ ] GPU preflight detects the supported NVIDIA or AMD/Intel path and fails
      before workload mutation when the requested path is unavailable.
- [ ] NVIDIA uses Docker GPU integration and requires the NVIDIA Container
      Toolkit; it does not manually mount host driver libraries.
- [ ] AMD/Intel exposes render nodes only and never the entire `/dev/dri`
      directory or KMS devices.
- [ ] Only integration-specific supplementary groups are added; all host
      groups are never inherited automatically.
- [ ] Rootless limitations and rootful fallback warnings are explicit.
- [ ] The integration never uses privileged mode, host namespaces, or broad
      device access.
- [ ] The explicit missing-integration override omits GPU access and records a
      warning without changing unrelated capabilities.
- [ ] GPU capability state affects desired-state matching and recreation.
- [ ] Tests cover NVIDIA prerequisites, render-node discovery, group mapping,
      unavailable hardware, omission behavior, and forbidden privilege flags.
- [ ] Hardware-conditional smoke checks are documented for hosts that provide
      NVIDIA or AMD/Intel devices.
