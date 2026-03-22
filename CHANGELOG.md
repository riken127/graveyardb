# Changelog

All notable changes to GraveyardDB will be documented here.

## Unreleased

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
