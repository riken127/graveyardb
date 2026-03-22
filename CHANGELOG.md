# Changelog

All notable changes to GraveyardDB will be documented here.

## Unreleased

### Added

* Release workflow now publishes versioned container images to GHCR for every `v*` tag.

### Changed

* Transition metadata is now mandatory on every event append; the server rejects writes that do not include `name`, `from_state`, and `to_state`.
* Runtime Docker image defaults now use `SCYLLA_KEYSPACE` to match server configuration.

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
