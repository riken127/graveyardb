# GraveyardDB

GraveyardDB is a distributed event store with ScyllaDB-backed primary storage, RocksDB fallback, gRPC APIs, sharded workers, schema support, snapshots, and language clients.

## Current Status

* The core Rust service handles append/read operations, schema management, snapshots, TLS, and token-based auth hooks.
* Cluster ownership is deterministic and based on the configured node list.
* Go, Java, and TypeScript SDKs exist under `sdks/`, but each should be validated against the release checklist before production use.
* Historical benchmark notes live in [BENCHMARKS.md](./BENCHMARKS.md); they are local-development measurements, not SLAs.

## Getting Started

### Prerequisites

* Rust stable
* Protocol Buffers compiler (`protoc`)
* Docker and Docker Compose for local cluster or ScyllaDB setups

### Run Locally

For RocksDB-only mode:

```bash
SCYLLA_KEYSPACE=graveyard cargo run --release
```

For ScyllaDB-backed mode:

```bash
SCYLLA_URI=127.0.0.1:9042 \
SCYLLA_KEYSPACE=graveyard \
CLUSTER_NODES=127.0.0.1:50051,127.0.0.1:50052 \
NODE_ID=0 \
PORT=50051 \
cargo run --release
```

Optional environment variables:
`REQUEST_TIMEOUT_MS`, `AUTH_TOKEN`, `TLS_CERT_PATH`, `TLS_KEY_PATH`, `DB_PATH`,
`SCHEMA_VALIDATION_HARD_FAIL`, `REQUIRE_TLS`, `REQUIRE_AUTH`, `OTEL_ENABLED`, `OTEL_FAIL_FAST`

### Local Cluster

```bash
docker-compose up -d
```

Then point the server at the local ScyllaDB endpoint and the node list you want to use.

## Release and Contribution

* Release process: [RELEASE.md](./RELEASE.md)
* Changelog: [CHANGELOG.md](./CHANGELOG.md)
* Contributing: [CONTRIBUTING.md](./CONTRIBUTING.md)

## SDKs

* Go: [sdks/go/README.md](./sdks/go/README.md)
* Java: [sdks/java/README.md](./sdks/java/README.md)
* TypeScript: [sdks/typescript/README.md](./sdks/typescript/README.md)
