# Backups and Recovery

GraveyardDB does not currently provide built-in backup orchestration. Operators must back up the underlying data stores directly.

## What to Back Up

* The RocksDB event store at `DB_PATH`.
* The RocksDB snapshot store at `${DB_PATH}_snapshots`.
* The ScyllaDB keyspace when Scylla-backed storage is in use.
* The deployed configuration, including `CLUSTER_NODES`, `NODE_ID`, `PORT`, TLS paths, and auth token handling.

## Why Both RocksDB Paths Matter

Snapshots are stored in a separate local RocksDB database from events. If you restore only the event store, the server can still operate, but you lose the saved snapshot state. If you restore only snapshots, you do not restore the event history they summarize.

## Backup Procedure

1. Quiesce writes or stop the node if you need a point-in-time copy.
2. Copy `DB_PATH` and `${DB_PATH}_snapshots` together.
3. If ScyllaDB is active, back up the keyspace that contains the `events` and `schemas` tables.
4. Record the release tag and runtime configuration that produced the backup.

## Recovery Procedure

### RocksDB Only

1. Stop the service.
2. Restore `DB_PATH` and `${DB_PATH}_snapshots` to the exact paths expected by the process.
3. Start the service with the same `SCYLLA_KEYSPACE`, `DB_PATH`, and cluster settings.
4. Run a read and snapshot smoke test before resuming traffic.

### Hybrid ScyllaDB Plus RocksDB

1. Restore the ScyllaDB keyspace first if the primary event store was lost.
2. Restore the local RocksDB event store and snapshot store.
3. Start the service with the same `SCYLLA_URI`, `SCYLLA_KEYSPACE`, and `DB_PATH`.
4. Confirm the service is not silently running in RocksDB-only fallback if that is unacceptable for your recovery target.

## Recovery Caveats

* The service does not automatically reconcile divergence between ScyllaDB and RocksDB fallback data.
* Snapshots are opaque payloads, so recovery must preserve the payload bytes exactly.
* There is no automatic replay-to-snapshot rebuild pipeline yet.
* Schema state should be restored together with event data to avoid validation drift.

