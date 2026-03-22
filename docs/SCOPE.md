# Project Scope and Purpose

## Purpose

GraveyardDB is an event store for append-only streams. The current codebase focuses on predictable writes, stream reads, schema registration, snapshots, and local failover.

The primary goals are:

* Performance: use Rust async, sharded workers, and storage engines that can handle write-heavy workloads.
* Simplicity: stay focused on append/read responsibilities instead of turning the store into a query engine.
* Reliability: keep stream ordering, version checks, and persistence behavior explicit.

## Scope

### In Scope
* Event appending through gRPC.
* Event reading through gRPC.
* Schema upsert and fetch.
* Server-side schema validation (primitives, enums, arrays, nested schemas, and field constraints).
* Snapshot save and fetch.
* Optimistic concurrency checks with `expected_version`.
* Pluggable storage with RocksDB and ScyllaDB backends.
* Deterministic stream ownership and request forwarding across a configured node list.
* Optional TLS and token-based auth hooks at the service boundary.
* Configurable schema enforcement mode via `SCHEMA_VALIDATION_HARD_FAIL`.

### Out of Scope

* Dynamic cluster membership and rebalancing.
* Full authN/authZ policy enforcement.
* Built-in projections or query models inside the store.
* Distributed transactions.

## Roadmap

1. Phase 0, Foundations: core API, RocksDB storage, basic pipeline, and documentation hygiene.
2. Phase 1, Hardening: observability, retry policy, failure testing, and SDK polish.
3. Phase 2, Cluster membership: discovery, rebalancing, and a live topology model.
