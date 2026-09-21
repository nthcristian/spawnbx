# 10: Add PipeWire Forwarding

**What to build:** Make PipeWire audio forwarding a strict, opt-in capability
that exposes only the selected PipeWire socket and preserves host-user access
semantics.

**Blocked by:** 04: Add Strict Preflight and Diagnostic Errors; 05: Implement
Managed Desired-State Reconciliation.

**Status:** ready-for-agent

- [ ] PipeWire preflight validates the selected remote/socket and host-user
      permissions before workload mutation.
- [ ] The workload receives only the PipeWire socket and required environment
      values.
- [ ] The whole host runtime directory, unrelated session sockets, and
      implicit PulseAudio or portal access are not mounted.
- [ ] Missing or inactive host audio sessions produce actionable diagnostics.
- [ ] The explicit missing-integration override omits PipeWire and records a
      warning without changing unrelated capabilities.
- [ ] PipeWire capability state affects desired-state matching and recreation.
- [ ] Tests cover socket validation, permission failures, omission behavior,
      and exact safe mount construction.
