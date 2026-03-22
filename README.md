# GraveyardDB

GraveyardDB is a distributed event store with ScyllaDB-backed primary storage, RocksDB fallback, gRPC APIs, sharded workers, schema support, mandatory transition metadata, snapshots, and language clients.

The core contract is `event -> transition -> snapshot`. Every append must include transition metadata on each event. Snapshots are saved and fetched explicitly; they are not auto-generated from appends.

## Quickstart

Start with [docs/QUICKSTART.md](./docs/QUICKSTART.md) for a runnable local setup.

Minimal examples:

```bash
SCYLLA_KEYSPACE=graveyard cargo run --release
```

```bash
SCYLLA_URI=127.0.0.1:9042 \
SCYLLA_KEYSPACE=graveyard \
CLUSTER_NODES=127.0.0.1:50051,127.0.0.1:50052 \
NODE_ID=0 \
PORT=50051 \
cargo run --release
```

`SCYLLA_KEYSPACE` is required even in RocksDB-only mode.

## Production Docs

* Production runbook: [docs/PRODUCTION_RUNBOOK.md](./docs/PRODUCTION_RUNBOOK.md)
* Configuration reference: [docs/CONFIGURATION_REFERENCE.md](./docs/CONFIGURATION_REFERENCE.md)
* API behavior and limits: [docs/API_BEHAVIOR.md](./docs/API_BEHAVIOR.md)
* Security model: [docs/SECURITY_MODEL.md](./docs/SECURITY_MODEL.md)
* Backups and recovery: [docs/BACKUPS_RECOVERY.md](./docs/BACKUPS_RECOVERY.md)
* Upgrade and release flow: [docs/UPGRADE_RELEASE_FLOW.md](./docs/UPGRADE_RELEASE_FLOW.md)
* SDK matrix: [docs/SDK_MATRIX.md](./docs/SDK_MATRIX.md)
* Architecture: [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)
* Scope: [docs/SCOPE.md](./docs/SCOPE.md)

## Current Behavior

* Append requests reject missing or empty transition metadata, invalid UUID event IDs, and unsupported `expected_version` values below `-1`.
* Append requests currently support exactly one event; multi-event batches are rejected.
* Schema validation is enforced only for event types with registered schemas; with hard-fail disabled, validation failures are logged and the append still proceeds.
* Stream ownership is deterministic and based on the sorted `CLUSTER_NODES` list; membership is static, not dynamically discovered.
* `REQUEST_TIMEOUT_MS` sets the timeout for inter-node forwarding calls; it does not cap local storage execution time.
* Snapshots live in a separate local RocksDB database at `${DB_PATH}_snapshots`, even when event storage is backed by ScyllaDB.

## SDKs

* Go: append/read/schema/snapshots, bearer auth, TLS, client timeouts, `ExpectedVersionAny`.
* Java: append/read/schema/snapshots, async append, bearer auth, TLS, client timeouts, `ANY_VERSION`.
* TypeScript: append/read/schema/snapshots, bearer auth, TLS, client timeouts, `ANY_VERSION`.
* Validate all SDKs against [CONTRIBUTING.md](./CONTRIBUTING.md) and [RELEASE.md](./RELEASE.md) before treating a release as production-ready.

## Release and Contribution

* Release process: [RELEASE.md](./RELEASE.md)
* Changelog: [CHANGELOG.md](./CHANGELOG.md)
* Contributing: [CONTRIBUTING.md](./CONTRIBUTING.md)

## Container Images

Tagged releases publish container images to GitHub Container Registry:

* `ghcr.io/<owner>/<repo>:vX.Y.Z`
* `ghcr.io/<owner>/<repo>:X.Y`
* `ghcr.io/<owner>/<repo>:latest`

You can pull an image and run it with your runtime configuration:

```bash
docker pull ghcr.io/<owner>/<repo>:vX.Y.Z
docker run --rm -p 50051:50051 \
  -e SCYLLA_KEYSPACE=graveyard \
  ghcr.io/<owner>/<repo>:vX.Y.Z
```
