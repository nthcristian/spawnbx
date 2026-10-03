# Spawnbx Agent Notes

## Source Of Truth

- The CLI flow is `src/lib.rs`; `src/contracts/` owns runtime, configurator, state-store, and parser interfaces; `src/adapters/` owns Docker, Nix, user, settings, and Clap implementations.
- `.spawnbx.yml` is required in the working directory. Only `name`, `shell`, and `packages` are read or saved; CLI options must precede a subcommand because they are not global.
- Wayland, PipeWire, and GPU flags apply only while creating a container. Wayland/PipeWire require host `XDG_RUNTIME_DIR`; Wayland also requires `WAYLAND_DISPLAY`. Recreate the container after changing these settings. X11 and networking controls remain unavailable.

## Behavior To Preserve

- `stop` still loads project settings to derive the container name, then stops the container through `DockerEnvironment` cleanup. `remove` runs `docker rm` and preserves `.spawnbx/home`; `recreate` removes before creating/configuring/attaching.
- Docker cleanup always attempts `docker container stop` and ignores its failure. `ensure_running` inspects before creating `.spawnbx/home`; retain this order when changing lifecycle code.
- Nix package reconciliation uses `/nix/var/nix/profiles/default`. The no-argument update request currently passes the literal `'.*'` argument to `nix profile upgrade`; preserve it unless the task explicitly changes update behavior.

## Verification

- Run `cargo fmt --check`, `cargo check --locked`, `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, then `cargo build --locked --release` for Rust changes.
- Unit-test bodies live in `tests/unit/` and are included from source with `#[path]`; `tests/cli.rs` uses fake `docker` and `id` executables, so it must not reach a real daemon.
- Runtime tests are intentionally ignored. Compile them with `cargo test --locked --test runtime --no-run`; run them only with a local `spawnbx:latest` image, a reachable Docker daemon, and `cargo test --locked --test runtime -- --ignored --test-threads=1`.

## Release And Metadata

- The npm workflow builds `target/release/spawnbx`, copies it to `bin/spawnbx`, and publishes on pushes to `main`; the container workflow publishes `ghcr.io/nthcristian/spawnbx` only when `Dockerfile` changes.
- `package.json` declares MIT while `LICENSE.md` contains GPL-3.0. Treat this as unresolved metadata; do not advertise or alter the license without an explicit decision.
