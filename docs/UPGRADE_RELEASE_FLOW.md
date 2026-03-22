# Upgrade and Release Flow

GraveyardDB uses semantic versioning and conventional commits.

## Versioning Rules

* `feat` usually maps to a minor version bump when the change is user-visible.
* `fix`, `docs`, and `chore` usually map to patch releases when they affect release notes or packaging.
* Breaking changes require a major version bump and a clear migration note.

## Operator Upgrade Flow

1. Read the changelog entry for the target version.
2. Confirm whether the release changes startup environment variables, API behavior, or SDK behavior.
3. Pin the image tag instead of relying on `latest`.
4. Roll out the new version to one node first if you want a canary.
5. Run the append, read, schema, and snapshot smoke tests.
6. Roll the remaining nodes only after the first node behaves as expected.

Because cluster membership is static, changing `CLUSTER_NODES` is effectively a topology change. Make that change deliberately and restart all nodes with the same ordered list.

## Release Flow

1. Update `CHANGELOG.md` under `Unreleased`.
2. Keep the release notes short, factual, and grouped by behavior.
3. Cut a release commit using conventional commit style.
4. Tag the commit as `vX.Y.Z`.
5. Push the commit and tag.
6. Let the release workflow verify the tagged build and publish the container image.
7. Confirm image availability in GHCR.

Tagged releases publish image tags for `vX.Y.Z`, `X.Y`, and `latest`.

## Release Hygiene

* Do not include generated build outputs in the release commit.
* If the release changes SDK behavior, update the corresponding SDK docs before tagging.
* If the release changes startup behavior or environment variables, update the root README and configuration docs at the same time.
* If the release changes the event, transition, or snapshot contract, update the API behavior doc before the tag is cut.

