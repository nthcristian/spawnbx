# spawnbx Development Overview

This folder turns the approved spawnbx architecture into an implementation
sequence. The repository is currently a Rust hello-world scaffold. The
architectural design is committed in `3b32400`; implementation has not begun.

## How To Use This

1. Start with ticket 01.
2. Work only on tickets whose blockers are complete.
3. Implement the end-to-end behavior and acceptance criteria in that ticket.
4. Run the ticket's verification checks before moving forward.
5. Mark the ticket complete here and in its file, then take the next frontier.
6. Keep the caller-facing `reconcile::run` Interface small; add behavior behind
   the existing Seam instead of adding orchestration methods to the CLI.

Every ticket is intended to fit in one implementation context and leave the
CLI in a more usable state. The Adapters are internal Seams: production
Adapters invoke the host, Docker, Nix, or terminal; fake Adapters make the
same reconciliation behavior testable without those dependencies.

Architecture view: [Mermaid module and type map](architecture.md).

## Execution Order

| Order | Ticket | Status | Blocked by |
|---:|---|---|---|
| 01 | [Build the default Arch/Nix image](issues/01-build-default-arch-nix-image.md) | Not started | None |
| 02 | [Implement the minimal spawnbx reconciliation path](issues/02-minimal-reconciliation-path.md) | Not started | 01 |
| 03 | [Add project configuration and CLI overrides](issues/03-project-configuration-overrides.md) | Not started | 02 |
| 04 | [Add strict preflight and diagnostic errors](issues/04-strict-preflight-diagnostics.md) | Not started | 03 |
| 05 | [Implement managed desired-state reconciliation](issues/05-managed-desired-state.md) | Not started | 04 |
| 06 | [Implement Nix profile and lock reconciliation](issues/06-nix-profile-lock.md) | Not started | 05 |
| 07 | [Add stop and remove lifecycle commands](issues/07-maintenance-lifecycle.md) | Not started | 06 |
| 08 | [Add X11 forwarding](issues/08-x11-forwarding.md) | Not started | 04, 05 |
| 09 | [Add Wayland forwarding](issues/09-wayland-forwarding.md) | Not started | 04, 05 |
| 10 | [Add PipeWire forwarding](issues/10-pipewire-forwarding.md) | Not started | 04, 05 |
| 11 | [Add GPU forwarding](issues/11-gpu-forwarding.md) | Not started | 04, 05 |
| 12 | [Complete the verified MVP](issues/12-verified-mvp.md) | Not started | 01-11 |

## Dependency Map

```text
01 Default image
 |
02 Minimal reconciliation
 |
03 Config and overrides
 |
04 Preflight and diagnostics
 |
05 Desired-state reconciliation
 |
06 Nix profile and lock
 |
07 Stop/remove commands
 |
 +--> 08 X11 --------+
 +--> 09 Wayland -----+
 +--> 10 PipeWire ----+--> 12 Verified MVP
 +--> 11 GPU ---------+
```

Tickets 08 through 11 can be implemented independently once 04 and 05 are
complete. The recommended implementation order remains numeric because each
ticket builds vocabulary and test fixtures used by the next one.

## Design Guardrails

- The command layer translates user input into an `Invocation`; it does not
  decide lifecycle ordering.
- `reconcile::run` is the only caller-facing orchestration Interface.
- Project, desired-state, preflight, lifecycle, package, and diagnostic
  modules own policy behind narrow Interfaces.
- Docker and Nix details stay inside their Adapters.
- Preflight must finish before workload creation, removal, replacement, or
  start.
- No privileged fallback, host networking, Docker socket, full runtime
  directory, or whole `/dev/dri` mount is allowed.
- Ordinary reconciliation respects the existing Nix lock.
- Package failures retain the managed workload for diagnosis.
- Tests cross the same reconciliation Interface as the CLI and replace only
  the internal Adapters.
- Do not add custom images, arbitrary Nix flakes/overlays, ARM support, or
  malicious-code sandboxing while implementing these tickets.

## Completion Checklist

- [ ] 01 complete
- [ ] 02 complete
- [ ] 03 complete
- [ ] 04 complete
- [ ] 05 complete
- [ ] 06 complete
- [ ] 07 complete
- [ ] 08 complete
- [ ] 09 complete
- [ ] 10 complete
- [ ] 11 complete
- [ ] 12 complete
