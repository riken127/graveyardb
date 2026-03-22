# Contributing to GraveyardDB

Thank you for contributing to GraveyardDB.

## Working Rules

* Keep commits small and focused.
* Use conventional commits, for example `docs: clarify release process` or `chore(ci): add release check`.
* Include a short commit body when the change needs context.
* Keep the repository clean. `git status --short` should be empty before you commit and after you finish a task.
* Do not commit generated outputs such as `target/`, `sdks/java/target/`, `sdks/typescript/dist/`, `sdks/typescript/node_modules/`, or local RocksDB files in `data/rocksdb/`.

## Getting Started

1. Install the required tools:
   * Rust stable
   * `protoc`
   * `make` for convenience commands
2. Run the relevant checks before you ask for review:
   * `cargo test`
   * `cargo clippy -- -D warnings`
   * `cargo fmt --all -- --check`
   * `cd sdks/go && go test ./...`
   * `cd sdks/typescript && npm test -- --runInBand`
   * `cd sdks/java && mvn -q test` when a backend is available for the integration test

## Release Flow

* Update `CHANGELOG.md` under `Unreleased`.
* Cut tagged releases as `vX.Y.Z`.
* Keep release notes short, factual, and based on conventional commits.

## Pull Requests

1. Make the change.
2. Run the relevant checks.
3. Verify no generated artifacts are staged.
4. Open the PR or, if you are working directly on `main`, commit with a clear conventional-commit message and description.

## Note on Shared Work

If you are touching a shared component, leave a short note in your commit body or handoff so the next person knows what changed and why.
