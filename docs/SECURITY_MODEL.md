# Security Model

GraveyardDB currently provides transport security and a simple bearer-token check. It does not yet provide a full authorization system.

## Transport Security

* TLS is optional at startup.
* If both `TLS_CERT_PATH` and `TLS_KEY_PATH` are present, the server starts with TLS enabled.
* If `REQUIRE_TLS=true`, startup fails unless both TLS paths are present.
* If TLS paths are missing and `REQUIRE_TLS=false`, the server starts in plaintext.

## Authentication

* When `AUTH_TOKEN` is configured, the server installs a gRPC interceptor that checks for `authorization: Bearer <token>`.
* If `REQUIRE_AUTH=true`, startup fails unless `AUTH_TOKEN` is present.
* The server checks token equality only. It does not parse claims, scopes, or roles.

## What Is Not Implemented

* No per-stream or per-method authorization.
* No role-based access control.
* No mTLS handshake policy.
* No token rotation API.
* No secret storage integration beyond environment variables.

## Operational Guidance

* Use TLS and bearer auth together for any network-exposed deployment.
* Protect the service with firewall rules, security groups, or a private network even when auth is enabled.
* Set `REQUIRE_TLS=true` and `REQUIRE_AUTH=true` in production so misconfiguration fails fast.
* Rotate the bearer token by redeploying with a new environment value.
* Treat plaintext mode as development-only unless a controlled internal network and external TLS termination are part of the deployment design.

