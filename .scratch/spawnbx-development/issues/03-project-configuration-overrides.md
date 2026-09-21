# 03: Add Project Configuration and CLI Overrides

**What to build:** Let users describe project intent in YAML while keeping the
fast flag-driven workflow. The same reconciliation path must resolve the
nearest project configuration, merge invocation overrides, optionally save the
merged result, and derive a stable readable workload name.

**Blocked by:** 02: Implement the Minimal spawnbx Reconciliation Path.

**Status:** ready-for-agent

- [ ] Starting from the current directory, the project module finds the
      nearest project YAML configuration while preserving the selected root.
- [ ] With no configuration, the documented defaults remain unchanged.
- [ ] YAML supports the approved fields: custom name, shell, bare Nix package
      attributes, X11, Wayland, PipeWire, GPU, and bridge/none network policy.
- [ ] Unknown fields, invalid types, invalid shell names, invalid package
      attributes, invalid network values, and malformed names produce focused
      configuration errors.
- [ ] CLI flags override configuration for one invocation.
- [ ] Package overrides replace the configured package list rather than
      appending implicitly.
- [ ] Explicit save writes the merged configuration atomically; a failed
      runtime preflight does not silently rewrite configuration unless save
      was requested.
- [ ] The workload name is the folder or custom name plus the first 12
      lowercase hexadecimal characters of the SHA-256 canonical project path.
- [ ] Name normalization produces valid Docker names and rejects an empty
      result.
- [ ] Tool-managed state remains under the canonical project root, including
      symlink and redirected-path checks.
- [ ] Tests cover nearest-configuration discovery, defaults, override
      precedence, save behavior, name stability, and unsafe paths.
