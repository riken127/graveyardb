# Release Process

GraveyardDB uses semantic versioning and conventional commits.

## Versioning Rules

* `feat` implies a minor version bump when the change is user-visible.
* `fix`, `docs`, and `chore` usually map to patch releases when they affect release notes or packaging.
* Breaking changes require a major version bump and a clear migration note.

## Changelog Flow

* Keep `CHANGELOG.md` up to date under `Unreleased`.
* Use conventional commit messages as the source material for the release notes.
* Keep release notes short, factual, and grouped by behavior.

## Release Checklist

1. Make sure the repository is clean with `git status --short`.
2. Run the relevant checks for the change set.
3. For infra or release changes, run Helm lint/template and the SDK checks that are impacted.
3. Update `CHANGELOG.md` with anything that should ship.
4. Cut a release commit using conventional commit style, for example `chore(release): v0.1.1`.
5. Tag the commit as `v0.1.1`.
6. Push the commit and tag.
7. Let the release workflow verify the tagged build and publish the container image.
8. Confirm image availability in GHCR under `ghcr.io/<owner>/<repo>`.

## Practical Notes

* Do not include generated build outputs in the release commit.
* If the release changes SDK behavior, update the corresponding SDK README before tagging.
* If the release changes startup or environment variables, update the root README at the same time.
* Release verification now includes `cargo build`, `cargo test`, Helm `lint` and `template`, plus Go, TypeScript, and Java SDK checks.
* Tagged releases publish image tags for `vX.Y.Z`, `X.Y`, and `latest`.
