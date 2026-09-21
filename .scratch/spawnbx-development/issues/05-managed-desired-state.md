# 05: Implement Managed Desired-State Reconciliation

**What to build:** Replace incidental Docker reuse with an explicit managed
state machine. spawnbx must label its workload with the effective desired
state, compare that state on every invocation, and safely choose reuse,
restart, recreation, creation, or collision failure.

**Blocked by:** 04: Add Strict Preflight and Diagnostic Errors.

**Status:** ready-for-agent

- [ ] Managed labels record project identity, effective desired-state hash,
      image digest, and state schema version.
- [ ] The effective desired state includes image, name, identity, mounts,
      shell, package intent, network, and granted integrations.
- [ ] Requested integrations omitted by the explicit missing-integration
      override are excluded from the effective hash and recorded as warnings.
- [ ] A matching running workload is reused.
- [ ] A matching stopped workload is started.
- [ ] A missing workload is created with the full desired specification.
- [ ] A managed workload with a changed desired state is stopped, removed, and
      recreated while project and persistent-home state remain intact.
- [ ] An unmanaged workload occupying the derived name is never changed and
      produces a collision error.
- [ ] Changing the image digest, identity, network, shell, package intent, or
      effective integration set produces the expected state transition.
- [ ] The lifecycle decision is a pure module with a small Interface and is
      tested independently from Docker command construction.
- [ ] Fake Workload Adapter tests assert the exact mutation order and prove
      that preflight failures never reach this state machine.
