# GraveyardDB Documentation

This directory documents current behavior, production operations, and the limits of the implementation.

The core lifecycle is `event -> transition -> snapshot`. Appends require transition metadata on every event, and snapshots are explicit state captures rather than an automatic side effect of writes.

## Documentation Map

### Start Here

* [Quickstart](QUICKSTART.md): Local setup, required environment variables, and a smoke test flow.

### Operate

* [Production Runbook](PRODUCTION_RUNBOOK.md): Deployment checklist, incident response, rollback guidance, and smoke tests.
* [Configuration Reference](CONFIGURATION_REFERENCE.md): Server environment variables and their current behavior.
* [Security Model](SECURITY_MODEL.md): TLS, bearer-token auth, and current security boundaries.
* [Backups and Recovery](BACKUPS_RECOVERY.md): What to back up and how to restore it.

### Behavior

* [API Behavior](API_BEHAVIOR.md): Event, transition, snapshot, schema, and append semantics.
* [Architecture](ARCHITECTURE.md): Module layout, routing, and storage flow.
* [Scope](SCOPE.md): What the project does today and what remains out of scope.

### Ship

* [SDK Matrix](SDK_MATRIX.md): Feature support across Go, Java, and TypeScript.
* [Upgrade and Release Flow](UPGRADE_RELEASE_FLOW.md): Tagged release flow and deployment considerations.
* [Release Process](../RELEASE.md): Semantic versioning, conventional commits, and changelog flow.
* [Changelog](../CHANGELOG.md): Release notes and unreleased changes.
* [Contributing](../CONTRIBUTING.md): Repository workflow and review expectations.

## Using These Docs

The docs are written against current code paths. When runtime behavior changes, update the relevant docs in the same patch so operators do not have to infer the new contract.
