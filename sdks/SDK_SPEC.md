# SDK Standardization Specification

This document outlines the standard architecture, patterns, and performance requirements for `graveyar_db` Client SDKs. All language implementations should adhere to these specifications to ensure consistency, reliability, and extreme performance.

## 1. Core Architecture

### Protocol
- **Transport**: gRPC over HTTP/2.
- **Serialization**: Protocol Buffers (Proxies for high-performance).
- **Service Definition**: `proto/api/eventstore.proto`.

### Implementation Layers
1.  **Low-Level gRPC Stub**: Generated directly from `protoc`. Do not modify.
2.  **Client Wrapper**: A high-level, idiomatic wrapper (e.g., Java Class, Go Struct).
    - **Responsibility**: Hides gRPC channels, implements timeouts, manages connection lifecycle.
3.  **Configuration**: Environment-aware configuration (Dev vs Prod).
    - **TLS**: Must support secure connections for Production. Plaintext defaults are acceptable only for local development and must be called out explicitly.
    - **Timeouts**: Must be configurable and should default to a finite per-RPC deadline.
    - **Auth**: Bearer-token or equivalent metadata-based auth should be plumbed through the wrapper when supported by the backend.
4.  **Entity/Schema Layer (ODM)**:
    - Reflection/Metaprogramming to map Language Objects to Proto Schemas.
    - Automatic Schema Registration via `UpsertSchema`.

## 2. Performance Requirements ("Extreme Speed")

To achieve low latency and high throughput:

- **Off-Heap Buffering**:
    - Use off-heap memory for network buffers where possible (e.g., Netty Direct Buffers in Java).
    - Minimized GC pressure by pooling objects.
- **Async I/O**:
    - All network operations MUST be non-blocking.
    - Expose `Async` / `Promise` / `Future` APIs to the user.
- **Connection Multiplexing**:
    - Use a single shared gRPC Channel/Connection for concurrent requests (HTTP/2 Multiplexing).
- **Zero-Copy**:
    - Aim for zero-copy deserialization path where language support exists.

## 3. Production Readiness Checklist

- [ ] **Optimistic Concurrency**: Must support `expected_version`.
- [ ] **Secure Transport**: Production configs must make TLS enablement obvious and document trust-store/CA behavior.
- [ ] **Schema Constraints**: Must validate data integrity using Proto constraints (Min/Max/Regex).
- [ ] **Resilience**: Retries (with backoff), Timeouts, and Circuit Breakers (optional but recommended).
- [ ] **Observability**: Expose metrics (request count, latency) and structured logs.

## 4. Implementation Steps (Reference: Java SDK)

1.  **Init**: Setup build system (Maven/Gradle/Make).
2.  **Proto**: Compile `eventstore.proto`.
3.  **Wrapper**: Implement `EventStoreClient`.
4.  **Config**: Implement `EventStoreConfig` (TLS/Timeout).
5.  **ODM**: Implement Annotations and Schema Generator.
6.  **Verify**: Strong Unit Tests with Mocking.

When documenting configuration, be explicit about the default transport mode. If an SDK defaults to plaintext for developer convenience, say so directly and show the production override that enables TLS, timeout controls, and auth metadata.

## 5. Schema & Entity Standard

SDKs must provide a way to define entities in code:

```java
@GraveyardEntity("user")
class User {
    @GraveyardField(min=18)
    int age;
}
```

And register them:

```java
client.upsertSchema(User.class);
```
