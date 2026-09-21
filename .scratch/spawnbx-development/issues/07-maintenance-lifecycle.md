# 07: Add Stop and Remove Lifecycle Commands

**What to build:** Complete the safe maintenance workflow around the managed
workload. Users must be able to stop or remove the workload deliberately
without losing project files, persistent home state, or the ability to
reconcile it later.

**Blocked by:** 06: Implement Nix Profile and Lock Reconciliation.

**Status:** ready-for-agent

- [ ] Stop targets only the managed workload resolved for the project.
- [ ] Remove deletes the managed workload but preserves project files,
      persistent home state, generated lock state, and configuration.
- [ ] Running stop/remove against an absent managed workload returns a clear,
      useful result rather than an opaque Docker error.
- [ ] Stop/remove refuse to modify an unmanaged workload with the derived
      name.
- [ ] A later bare invocation recreates or starts the workload and reuses the
      preserved project/home state.
- [ ] Partial Docker failures report whether workload state may have changed
      and do not retry automatically.
- [ ] Tests cover running, stopped, absent, managed, and unmanaged workload
      states through the Workload Adapter Interface.
