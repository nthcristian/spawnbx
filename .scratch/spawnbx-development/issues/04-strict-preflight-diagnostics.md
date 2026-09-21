# 04: Add Strict Preflight and Diagnostic Errors

**What to build:** Make host readiness a first-class policy module. Before any
workload creation, removal, replacement, or start, spawnbx must validate the
platform, Docker mode, identity, project state, network policy, and requested
host resources it currently knows about. Add the non-mutating `doctor` path
and a stable diagnostic model that later desktop and GPU tickets extend.

**Blocked by:** 03: Add Project Configuration and CLI Overrides.

**Status:** ready-for-agent

- [ ] Preflight runs after desired configuration is resolved and before any
      workload mutation.
- [ ] Preflight validates Linux, `linux/amd64`, Docker availability, image
      compatibility, rootless/rootful mode, host UID/GID, and safe project
      state paths.
- [ ] Rootless Docker is preferred; rootful fallback emits an explicit
      warning; no privileged fallback exists.
- [ ] Bridge and `none` network policies are validated without exposing host
      networking or arbitrary network modes.
- [ ] Independent preflight failures are aggregated into one actionable
      result rather than requiring one fix per invocation.
- [ ] `--allow-missing-integrations` can omit an unavailable requested
      integration only with an explicit warning and never broadens privileges.
- [ ] `doctor` performs discovery, configuration, and preflight but does not
      pull an image, create/remove/start/stop a workload, change Nix state, or
      attach a shell.
- [ ] Errors preserve phase, stable category, exit status when available,
      stdout, stderr, and remediation without exposing unsafe secrets.
- [ ] Fake Host and Workload Adapters prove that every preflight failure leaves
      workload state untouched.
