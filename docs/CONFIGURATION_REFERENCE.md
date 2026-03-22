# Configuration Reference

This page documents the current server environment variables and how the running binary uses them today.

## Server Runtime

| Variable | Required | Default | Current Behavior |
| --- | --- | --- | --- |
| `SCYLLA_URI` | No | unset | When present and ScyllaDB connects successfully, the server uses Scylla as the primary event store and RocksDB as the fallback. If the connection fails, startup continues with RocksDB only. |
| `SCYLLA_KEYSPACE` | Yes | none | Required at startup even in RocksDB-only mode. Used for the Scylla event and schema tables when ScyllaDB is enabled. |
| `REQUEST_TIMEOUT_MS` | No | `3000` | Deadline for forwarded append calls between cluster nodes. It does not cap local storage execution time on the receiving node. |
| `NODE_ID` | No | `0` | Chooses the advertised local address from the sorted `CLUSTER_NODES` list. If the index is out of bounds, the server falls back to the first configured node. |
| `CLUSTER_NODES` | No | `127.0.0.1:50051` | Static node list used for deterministic stream ownership and forwarding. The list is sorted at startup. |
| `PORT` | No | `50051` | gRPC listen port. |
| `DB_PATH` | No | `data/rocksdb` | RocksDB path for event storage when RocksDB is active. Snapshot data is stored in a separate RocksDB database at `${DB_PATH}_snapshots`. |
| `AUTH_TOKEN` | No | unset | Enables the bearer-token auth interceptor. If `REQUIRE_AUTH=true` and this is missing, startup fails. |
| `TLS_CERT_PATH` | No | unset | When paired with `TLS_KEY_PATH`, enables TLS on the gRPC server. If `REQUIRE_TLS=true` and either TLS path is missing, startup fails. |
| `TLS_KEY_PATH` | No | unset | See `TLS_CERT_PATH`. |
| `SCHEMA_VALIDATION_HARD_FAIL` | No | `false` | When `true`, schema validation failures reject the append. When `false`, validation failures are logged and the append continues. |
| `REQUIRE_TLS` | No | `false` | Startup guardrail that requires both TLS paths to be present. |
| `REQUIRE_AUTH` | No | `false` | Startup guardrail that requires `AUTH_TOKEN` to be present. |
| `OTEL_ENABLED` | No | `false` | Attempts OpenTelemetry exporter initialization. If initialization fails and `OTEL_FAIL_FAST=false`, the server continues with local tracing only. |
| `OTEL_FAIL_FAST` | No | `false` | If `true`, OpenTelemetry exporter initialization failure stops startup. |

## Practical Notes

* `SCYLLA_KEYSPACE` is always required because the binary validates it before deciding whether to use ScyllaDB or RocksDB only.
* `REQUEST_TIMEOUT_MS` applies to cluster forwarding. Keep client-side deadlines for user-facing SDK calls.
* `DB_PATH` must point at persistent storage if you want to preserve event data across restarts. The snapshot database uses a sibling path and needs persistence too.
* `CLUSTER_NODES` is not a dynamic membership system. Change it carefully and restart all nodes with the same list.
