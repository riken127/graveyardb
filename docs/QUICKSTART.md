# Quickstart

## Prerequisites

* Rust stable
* Protocol Buffers compiler (`protoc`)
* Docker and Docker Compose for local cluster or ScyllaDB setups

`SCYLLA_KEYSPACE` is required by server startup even when you only want RocksDB-backed local storage.

## Run Locally

### RocksDB Only

```bash
export SCYLLA_KEYSPACE=graveyard
cargo run --release
```

### ScyllaDB Backed

```bash
export SCYLLA_URI=127.0.0.1:9042
export SCYLLA_KEYSPACE=graveyard
export CLUSTER_NODES=127.0.0.1:50051,127.0.0.1:50052
export NODE_ID=0
export PORT=50051
cargo run --release
```

### Local Cluster

```bash
docker-compose up -d
```

Then point the server at the local ScyllaDB endpoint and the node list you want to use.

## Smoke Test

Send a single-event append that includes transition metadata. The wire
`payload` field is bytes, so serialize the JSON body before sending it:

```json
{
  "stream_id": "user-123",
  "expected_version": -1,
  "events": [
    {
      "id": "7a1b1e2f-7d11-4b7c-9f3c-6d3fa7a7f2d0",
      "event_type": "UserCreated",
      "payload": "{\"name\":\"Ada\"}",
      "timestamp": 1711111111111,
      "transition": {
        "name": "user.created",
        "from_state": "draft",
        "to_state": "active"
      }
    }
  ]
}
```

Then read the stream back and, if needed, save a snapshot at the resulting version.

## Next Steps

* Review [Configuration Reference](CONFIGURATION_REFERENCE.md) before deploying outside a laptop.
* Read [API Behavior](API_BEHAVIOR.md) for the append, schema, and snapshot contract.
* Use [Production Runbook](PRODUCTION_RUNBOOK.md) for deployment and rollback checks.
