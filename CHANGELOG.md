# Changelog

All notable changes to GraveyardDB will be documented here.

## Unreleased

No unreleased changes yet.

## 0.3.0 - 2026-03-22

### Added

* Core append-path hardening: single-event append enforcement, fail-closed schema lookup, forwarding timeout wiring, and TLS-aware peer URI handling.
* Snapshot write guardrails that reject stale or ahead-of-stream versions.
* SDK parity features: Go snapshot helpers, Java schema lookup helper, and TypeScript schema/snapshot helpers.
* Production deployment hardening in Helm: auth secret template, PodDisruptionBudget, probes, pod/container security contexts, and placement constraints.
* CI and release verification coverage for Helm plus Go, TypeScript, and Java SDK checks.
* Production-grade product/operator documentation suite (quickstart, runbook, configuration reference, API behavior, security model, backups/recovery, upgrade flow, SDK matrix).

### Changed

* Transition metadata is mandatory across runtime and SDK append calls.
* Schema contract validation is stricter and rejects invalid constraint combinations before persistence.
* Constraint evaluation now applies `min_length` and `max_length` to arrays and uses character-count semantics for string lengths.
* Event type strings are preserved end-to-end for correct schema lookup and validation.
* Runtime container defaults are hardened (non-root execution, healthcheck tooling, and aligned keyspace/data path defaults).
* Release workflow publishes versioned container images to GHCR for every `v*` tag.

### Fixed

* Local optimistic concurrency checks and recursive schema validation edge cases.
* Java SDK transport-layer testability and timeout/auth behavior coverage.

### Removed

* Tracked generated TypeScript build/dependency artifacts (`sdks/typescript/dist`, `sdks/typescript/node_modules`) from git.
* Tracked local RocksDB data directories from git.

## 0.2.0 - 2026-03-22

### Added

* Structured pipeline error mapping and targeted append-path tests in the Rust core.
* Configurable strict schema enforcement with `SCHEMA_VALIDATION_HARD_FAIL`.
* Startup hardening toggles: `REQUIRE_TLS`, `REQUIRE_AUTH`, `OTEL_ENABLED`, and `OTEL_FAIL_FAST`.
* Release process guidance and changelog workflow docs.
* TypeScript SDK root exports and subpath package exports.
* Go SDK auth metadata support and expanded schema-tag parsing.

### Changed

* `expected_version` contract is now consistently signed (`int64`) with `-1` as the no-check sentinel across service and SDKs.
* Java SDK tests were refactored to use a transport test double; integration tests are now opt-in by environment flag.
* Repository documentation now reflects current behavior and release practices.

### Removed

* Tracked generated TypeScript build/dependency artifacts (`sdks/typescript/dist`, `sdks/typescript/node_modules`) from git history going forward.
