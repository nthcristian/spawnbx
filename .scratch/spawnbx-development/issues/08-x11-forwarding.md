# 08: Add X11 Forwarding

**What to build:** Make the X11 integration a strict, opt-in capability that
can be requested from YAML or the CLI and is realized through the existing
preflight, desired-state, and workload Seams.

**Blocked by:** 04: Add Strict Preflight and Diagnostic Errors; 05: Implement
Managed Desired-State Reconciliation.

**Status:** ready-for-agent

- [ ] X11 preflight validates the display environment and required socket
      paths before any workload mutation.
- [ ] The workload receives `DISPLAY` and only the required X11 socket mount.
- [ ] Relevant authorization data is mounted read-only when required.
- [ ] The integration never uses `xhost +`, mounts the whole runtime
      directory, or grants unrelated host sockets.
- [ ] Missing display/session/authorization access fails clearly by default.
- [ ] The explicit missing-integration override omits X11 and records a
      warning without changing other grants.
- [ ] X11 capability state affects desired-state matching and causes workload
      recreation when it changes.
- [ ] Tests assert exact effective grants and prove no broad runtime mount or
      privileged flag can be produced.
