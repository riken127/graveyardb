# Future Work & Roadmap

GraveyardDB currently exposes the core event-store path, but it is still being hardened for production use. This file is the real backlog, not a completion claim.

## Current Surface

* Append and read stream APIs.
* Schema upsert and fetch APIs.
* Snapshot save and fetch APIs.
* Hybrid RocksDB and ScyllaDB storage.
* Deterministic cluster forwarding with a static node list.
* Go, Java, and TypeScript SDKs.

## Recently Completed (v0.2.0 Track)

* Standardized `expected_version` semantics (`-1` sentinel) across server and SDKs.
* Added structured append error mapping for better gRPC status responses.
* Added `SCHEMA_VALIDATION_HARD_FAIL` toggle for strict schema enforcement.
* Added deployment guardrails: `REQUIRE_TLS` and `REQUIRE_AUTH`.
* Made OpenTelemetry startup opt-in (`OTEL_ENABLED`) with optional fail-fast (`OTEL_FAIL_FAST`).
* Added release docs/checklists and changelog workflow.
* Removed tracked generated TypeScript artifacts from the repository.

## Remaining Backlog

### 1. Observability
* Add metrics for request counts, latency, storage fallback, and forwarding.

### 2. Cluster Membership
* Replace `CLUSTER_NODES` with discovery or membership updates.
* Rebalance stream ownership when the cluster topology changes.

### 3. Snapshotting and Replay
* Make snapshot cadence configurable.
* Use snapshots to shorten replay for long streams.

### 4. Retention and Compaction
* Define TTL or retention rules for old events and schema history.
* Make soft deletion and cleanup policy explicit.

### 5. Query and Projection Support
* Keep projections in a separate read-model component rather than inside the core store.
* Document the event-contract expectations for downstream consumers.

### 6. Security
* Add stronger TLS defaults beyond optional startup guards.
* Replace the current token hook with a fuller authN/authZ model.

### 7. Testing
* Add property-based tests for the event store trait.
* Add multi-node and failure-focused tests for partitions and failover.

## Release Hygiene

* Every release should come from a tagged commit.
* Every release should have a changelog entry.
* Release notes should be assembled from conventional commits.
