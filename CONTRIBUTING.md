# Contributing

## Development Flow

1. Start from an up-to-date `main` branch.
2. Create a focused branch for the change.
3. Keep implementation and documentation changes narrowly scoped.
4. Run the relevant checks before opening a pull request.
5. Explain the behavior change and verification in the pull request description.

Use clear imperative commit messages, for example:

```text
fix: report image pull failures
docs: clarify project configuration
```

## Checks

Run these checks for Rust changes:

```sh
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

For container changes, build the image locally when Docker is available:

```sh
docker build --platform linux/amd64 -t spawnbx:local .
```

The published workflows also validate the npm package and create a container from the pushed GHCR image.

### Opt-In Runtime Tests

Default tests are isolated; real Docker/Nix tests are ignored. To compile them without executing:

```sh
cargo test --locked --test runtime --no-run
```

Runtime tests require a non-root host user with nonzero UID/GID, a reachable local Docker daemon with matching bind-mount ownership (not a remote daemon or remapped/rootless setup), and an existing debug image tagged `spawnbx:latest`. The image is Linux/amd64; other hosts need compatible emulation. Prepare it explicitly if needed:

```sh
docker build --platform linux/amd64 -t spawnbx:latest .
cargo test --locked --test runtime -- --ignored --test-threads=1
```

Tests never build or pull images, and missing prerequisites fail rather than silently skip. The Nix test installs the small `nixpkgs#hello` package, requiring registry/network access or sufficient cached inputs; use a baseline image without `hello` already installed. Do not use `--release`, which selects the published image instead.

Coverage includes create/reuse/restart/remove, repeated user setup, mapped UID/GID execution and persisted home ownership, and Nix installation with an unchanged profile/generation on repeat. Tests use uniquely named `spawnbx-runtime-*` containers and temporary workspaces/homes, never repository containers or the host home. An independent guard checks the exact name is unused before owning it, retains the workspace through cleanup, and removes the test container even after production Drop stops it. Abrupt process termination or daemon/cleanup failures can still leave disposable resources; remove only the exact test-owned resources reported.

Runtime failures in the unchanged user/group scripts remain failures. Interactive attach, the existing attach/stop `todo!()` panic, and the invalid literal quoted update-all argument `'.*'` are not successful runtime workflows and remain preserved limitations. These tests do not establish lock-file reproducibility.

## Generated Files

Do not commit local build or runtime state:

- `target/`
- `.spawnbx/`
- `bin/spawnbx`
- Nix `result` symlinks

Keep generated files out of pull requests unless a file is explicitly part of the source contract.

## Pull Requests

Pull requests should include:

- A short description of the problem and solution.
- Tests or checks that were run.
- Any platform, Docker, Nix, or release implications.
- Follow-up work when the change intentionally leaves a limitation.

Avoid unrelated formatting or refactoring in the same pull request.

## Releases

Pushes to `main` publish the npm binary. The npm workflow compares `package.json` with the published `spawnbx` version, bumps the patch version when needed, commits that version bump with `[skip ci]`, and publishes the binary.

The container workflow publishes `ghcr.io/nthcristian/spawnbx:latest` only when `Dockerfile` changes on `main`.

Release-related changes should preserve the npm trusted publisher configuration and the `NPM_PUBLISH` GitHub environment. Do not add registry tokens or other credentials to the repository.
