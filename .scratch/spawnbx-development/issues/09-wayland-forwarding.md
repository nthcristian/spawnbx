# 09: Add Wayland Forwarding

**What to build:** Make Wayland a strict, opt-in capability using the existing
preflight and desired-state policy, with only the selected user session socket
available inside the workload.

**Blocked by:** 04: Add Strict Preflight and Diagnostic Errors; 05: Implement
Managed Desired-State Reconciliation.

**Status:** ready-for-agent

- [ ] Wayland preflight validates `WAYLAND_DISPLAY`, the selected socket, and
      the host-user permission relationship before workload mutation.
- [ ] The workload receives the selected Wayland socket and a container-local
      runtime directory without exposing the entire host runtime directory.
- [ ] Socket disappearance, logout, and permission mismatch produce focused
      diagnostics.
- [ ] The integration never mounts unrelated D-Bus, SSH-agent, GPG-agent, or
      other runtime sockets.
- [ ] The explicit missing-integration override omits Wayland and records a
      warning without broadening access.
- [ ] Wayland capability state affects desired-state matching and recreation.
- [ ] Tests cover valid sockets, missing sockets, permissions, omitted
      integrations, and safe mount construction.
