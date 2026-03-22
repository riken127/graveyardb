# API Behavior

This page describes the current behavior of the gRPC API and the limitations operators should expect.

## Core Contract

GraveyardDB stores immutable events. Each event carries:

* `id`
* `event_type`
* `payload`
* `metadata`
* `transition { name, from_state, to_state }`

The transition is mandatory. The server rejects events that do not provide it, or that provide empty transition fields, or that use the same `from_state` and `to_state`.

Event IDs are parsed as UUIDs. Invalid UUID strings are rejected before persistence.

`event_type` is preserved as a string end to end. The server does not force the value into a closed enum, so custom event type strings remain intact.

## Append Semantics

* `expected_version = -1` disables the optimistic concurrency check.
* Any other negative value is rejected as invalid input.
* Non-negative values require an exact match with the current stream version.
* Append requests currently support exactly one event; requests with more than one event are rejected.
* The append pipeline checks stream ownership before it writes.
* When a request is forwarded to another node, the internal `is_forwarded` flag prevents the request from being forwarded again.

## Schema Semantics

* `UpsertSchema` validates the schema contract before persistence.
* Invalid contract definitions are rejected with `INVALID_ARGUMENT`.
* If a schema exists for an event type, payload validation runs against that schema.
* Payload validation expects JSON bytes. If the payload is not valid JSON, validation fails.
* When `SCHEMA_VALIDATION_HARD_FAIL=false`, validation failures are logged and the append still proceeds.
* When `SCHEMA_VALIDATION_HARD_FAIL=true`, validation failures reject the append.
* Schema upsert persists both the schema history stream and the latest schema projection.

## Read Semantics

* `GetEvents` returns events ordered by sequence number.
* Missing streams return an empty stream rather than an error.
* `GetSchema` returns `found=false` when no schema exists.
* `GetSnapshot` returns `found=false` when no snapshot exists.

## Snapshot Semantics

* `SaveSnapshot` is an explicit RPC. The server does not create snapshots automatically after appends.
* Snapshot payloads are opaque bytes. The server stores them as provided and does not interpret the payload.
* Snapshot version must match the current stream version.
* Snapshot writes older than an existing stored snapshot are rejected.
* The snapshot store is local RocksDB state, even when ScyllaDB is used for event storage.

## Ownership and Forwarding

* Stream ownership is determined by hashing the stream ID against the sorted `CLUSTER_NODES` list.
* Membership is static. The system does not discover nodes dynamically or rebalance automatically.
* If a node receives an append for a stream it does not own, it forwards the request to the owner.
* If forwarding fails, the RPC returns `UNAVAILABLE`.

## Status Mapping

* Invalid input, including malformed transitions and unsupported `expected_version` values, returns `INVALID_ARGUMENT`.
* Ownership or schema-validation failures return `FAILED_PRECONDITION`.
* Concurrency conflicts return `ABORTED`.
* Forwarding failures return `UNAVAILABLE`.
* Storage failures return `INTERNAL`.

## Known Limitations

* No multi-event append support yet; appends are single-event only.
* No automatic snapshot generation or retention policy.
* No live cluster membership or rebalancing.
* `REQUEST_TIMEOUT_MS` currently applies to forwarded append RPC deadlines, not to local storage execution time.
