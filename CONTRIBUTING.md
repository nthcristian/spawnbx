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
cargo test
cargo build --locked --release
```

For container changes, build the image locally when Docker is available:

```sh
docker build --platform linux/amd64 -t spawnbx:local .
```

The published workflows also validate the npm package and create a container from the pushed GHCR image.

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
