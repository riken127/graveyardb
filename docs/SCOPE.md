# Project Scope and Purpose

## Purpose

GraveyardDB is an event store for append-only streams. The current codebase focuses on predictable writes, stream reads, schema registration, transitions, snapshots, and local failover.

The core lifecycle is `event -> transition -> snapshot`. Every event must include transition metadata (`name`, `from_state`, `to_state`); appends without it are rejected.

The primary goals are:

* Performance: use Rust async, sharded workers, and storage engines that can handle write-heavy workloads.
* Simplicity: stay focused on append/read responsibilities instead of turning the store into a query engine.
* Reliability: keep stream ordering, version checks, and persistence behavior explicit.

## Scope

### In Scope
* Event appending through gRPC with required transition metadata on every event.
* Event reading through gRPC.
* Schema upsert and fetch.
* Server-side schema contract validation on upsert (coherent and type-safe constraints) plus payload validation for registered schemas.
* Snapshot save and fetch as explicit operations.
* Optimistic concurrency checks with `expected_version`.
* Pluggable storage with RocksDB and ScyllaDB backends, with best-effort fallback to RocksDB when ScyllaDB is unavailable.
* Deterministic stream ownership and request forwarding across a configured node list.
* Optional TLS and bearer-token auth hooks at the service boundary.
* Configurable schema enforcement mode via `SCHEMA_VALIDATION_HARD_FAIL`.

### Out of Scope

* Dynamic cluster membership and rebalancing.
* Full authN/authZ policy enforcement.
* Built-in projections or query models inside the store.
* Distributed transactions.
* Automatic snapshotting or replay orchestration.
* Transactional multi-event append batches.

### Current Constraints

* Transition metadata is mandatory and validated before persistence.
* Event payload validation only runs when a schema exists for the event type.
* When schema hard-fail is disabled, payload validation warnings do not block the append.
* The snapshot store is local RocksDB state, not a replicated cluster-wide snapshot system.

## Roadmap

1. Phase 0, Foundations: core API, RocksDB storage, basic pipeline, and documentation hygiene.
2. Phase 1, Hardening: observability, retry policy, failure testing, and SDK polish.
3. Phase 2, Cluster membership: discovery, rebalancing, and a live topology model.
