# 06: Implement Nix Profile and Lock Reconciliation

**What to build:** Install project packages through Nix inside the managed
workload and make the persistent profile deterministic through generated lock
state. Package declarations stay in YAML; generated Nix metadata and the lock
remain tool-managed under the project state directory.

**Blocked by:** 05: Implement Managed Desired-State Reconciliation.

**Status:** ready-for-agent

- [ ] Bare nixpkgs attribute paths are normalized deterministically and are
      never treated as shell fragments.
- [ ] The package module generates deterministic Nix metadata from effective
      package intent.
- [ ] First package reconciliation creates the lock and installs the
      aggregate target into a dedicated profile under persistent home state.
- [ ] Ordinary reconciliation respects the existing lock and never advances
      Nixpkgs implicitly.
- [ ] Package additions and removals reconcile the desired profile; removed
      packages are no longer profile members even if store paths remain.
- [ ] Recreating the workload reuses the persistent Nix state and repairs the
      profile when necessary.
- [ ] The update command advances the shared Nixpkgs input; a specific package
      argument identifies the requested result but does not promise isolated
      package pinning.
- [ ] Lock updates are staged or recoverable when the update or profile
      operation fails, and the managed workload remains available for diagnosis.
- [ ] The selected shell is checked after profile reconciliation and fails with
      an actionable error if it is not installed and executable.
- [ ] Nix process output, exit status, and diagnostics are preserved through
      the Package Adapter without exposing Nix command construction to the
      reconciliation caller.
- [ ] Tests cover first lock creation, locked reinstall, package removal,
      update behavior, invalid attributes, profile failure, and shell failure.
