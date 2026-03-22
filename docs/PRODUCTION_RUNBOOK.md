# Production Runbook

This runbook is for operators who need to deploy, verify, and recover a running GraveyardDB instance.

## Preconditions

* Choose a tagged image or release commit and keep the tag pinned.
* Ensure persistent storage exists for `DB_PATH` and `DB_PATH_snapshots`.
* Decide whether the deployment should tolerate a ScyllaDB outage falling back to RocksDB, because that fallback is automatic today.
* Decide whether missing TLS or auth should fail startup. If so, set `REQUIRE_TLS=true` and `REQUIRE_AUTH=true`.
* Confirm the `CLUSTER_NODES` list is identical on every node in the cluster.
* Confirm the SDKs you are using are compatible with the release you are deploying.

## Deployment Checklist

1. Set the server environment from [Configuration Reference](CONFIGURATION_REFERENCE.md).
2. Provision storage for the event store and snapshot store before first boot.
3. Start one node, confirm it reaches the expected storage backend, then roll out the remaining nodes.
4. Verify logs show the expected security mode:
   * TLS enabled when cert and key paths are present.
   * Auth enabled when `AUTH_TOKEN` is present.
5. Run a smoke test:
   * append an event with transition metadata,
   * read the stream back,
   * save a snapshot,
   * fetch the snapshot.

## What to Watch

* Append failures with `ABORTED` usually mean an `expected_version` mismatch.
* `FAILED_PRECONDITION` on append usually means ownership mismatch or schema validation failure.
* `UNAVAILABLE` on append usually means forwarding to the owner node failed.
* Schema validation warnings without hard-fail mean the write still succeeded.
* OpenTelemetry startup failures are non-fatal unless `OTEL_FAIL_FAST=true`.

## Incident Response

* If ScyllaDB is unavailable, check whether the node has fallen back to RocksDB-only operation.
* If that fallback is not acceptable for the incident, stop traffic and restore the primary database before resuming writes.
* If snapshots are missing, restore `DB_PATH_snapshots` from backup and verify the stream version before trusting the snapshot.
* If a token or certificate change breaks startup, fix the environment and restart the node.

## Rollback

1. Stop the node or deployment.
2. Restore the previous tagged image.
3. Restore `DB_PATH` and `DB_PATH_snapshots` if the data layout changed or became inconsistent.
4. Re-run the smoke test before putting the node back into service.

## Routine Checks

* Confirm the server still answers append and read requests.
* Confirm new snapshots are readable after save.
* Confirm backups are being captured for both event data and snapshots.
* Confirm the release notes for the deployed tag match the behavior you are relying on.
