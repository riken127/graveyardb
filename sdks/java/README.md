# EventStore Java SDK

Java client library for `graveyar_db` using gRPC. Supports Spring Boot, async operations, optimistic concurrency control, TLS, and bearer-token auth.

## Features

- **Protocol**: gRPC with Protobuf.
- **Concurrency**: Optimistic locking via `expectedVersion`.
- **Resilience**: Configurable Timeouts.
- **Performance**: Async API (`ListenableFuture`) and Sync API.
- **Integration**: Spring Boot `@Service` and `@Configuration`.
- **Environment**: Easy toggle between Plaintext (Dev), TLS (Prod), and bearer-token auth.

## Installation

Add the following to your `pom.xml` (assuming local install):

```xml
<dependency>
    <groupId>com.eventstore</groupId>
    <artifactId>eventstore-client</artifactId>
    <version>0.0.1-SNAPSHOT</version>
</dependency>
```

## Configuration

Configure the client in your `application.properties` or `application.yml`:

| Property | Default | Description |
|----------|---------|-------------|
| `eventstore.host` | `localhost` | Hostname of the EventStore server. |
| `eventstore.port` | `50051` | gRPC port. |
| `eventstore.use-tls` | `false` | Plaintext for local development; set `true` for production TLS with the JVM trust store. |
| `eventstore.auth-token` | empty | Optional bearer token sent as `authorization: Bearer <token>` on outgoing requests. |
| `eventstore.timeout-ms` | `5000` | Default per-RPC timeout in milliseconds. |

The SDK also exposes the same defaults as a plain Java object via `EventStoreConfig`, so you can use it outside Spring if you prefer.

## Usage

Inject the client into your service:

```java
@Autowired
private EventStoreClient client;
```

### Entity & Schema Management

Annotate your domain objects:

```java
@GraveyardEntity("user_profile")
public class UserProfile {
    @GraveyardField(minLength = 3, regex = "^[a-z]+$", nullable = false)
    private String username;
    
    @GraveyardField(min = 18, max = 150)
    private int age;
}
```

Supported Constraints:
- `min` / `max`: For numeric values.
- `minLength` / `maxLength`: For strings.
- `regex`: Regular expression pattern. The Java `SchemaValidator` can check this locally, but backend enforcement depends on server support.
- `nullable`: Whether the field is optional (default: true).

The schema generator exports declared instance fields. `@GraveyardField` controls nullability and constraint metadata; unannotated fields are included as nullable, unconstrained schema fields. `transient` and `static` fields are skipped.

Register the schema:

```java
client.upsertSchema(UserProfile.class);
```

Lookup a schema or snapshot when you need read-side parity with the service:

```java
GetSchemaResponse schemaResponse = client.getSchema("user_profile");
Snapshot snapshot = client.getSnapshot("user-123");
boolean saved = client.saveSnapshot("user-123", 42L, new byte[0], System.currentTimeMillis());
```

### Append Sync

The snippet assumes `Event` and `Transition` are imported from `com.eventstore.client.model`.

```java
List<Event> events = List.of(Event.newBuilder()
    .setId("1")
    .setEventType("UserCreated")
    .setTransition(Transition.newBuilder()
        .setName("UserCreated")
        .setFromState("draft")
        .setToState("active")
        .build())
    .build());
// Use EventStoreClient.ANY_VERSION for "append regardless of current stream version".
boolean success = client.appendEvent("stream-1", events, EventStoreClient.ANY_VERSION);
```

`EventStoreClient.ANY_VERSION` maps to the server's `expected_version = -1` sentinel. Any other negative value is rejected by the client before the request is sent.
Every appended event must include a non-empty transition name, `from_state`, and `to_state`, and `from_state` must differ from `to_state`.

### Append Async

```java
ListenableFuture<AppendEventResponse> future = client.appendEventAsync("stream-1", events, 10);
Futures.addCallback(future, new FutureCallback<>() {
    public void onSuccess(AppendEventResponse r) { ... }
    public void onFailure(Throwable t) { ... }
}, executor);
```

### Read Stream

```java
Iterator<Event> events = client.getEvents("stream-1");
while (events.hasNext()) {
    Event e = events.next();
    System.out.println(e.getPayload().toStringUtf8());
}
```

## Development

Build and run tests:

```bash
mvn test
```

Integration coverage is opt-in so local and CI runs stay green without a live backend:

```bash
EVENTSTORE_INTEGRATION_TESTS=true EVENTSTORE_HOST=localhost EVENTSTORE_PORT=50051 mvn test
```

## Production Guide

### Performance
The client uses gRPC-Netty, which manages off-heap buffers for high-performance I/O. To maximize throughput:
- **Reuse Client**: Create one `EventStoreClient` bean and share it across threads. It is thread-safe and uses a single multiplexed connection.
- **Use Async**: Prefer `appendEventAsync` for high-volume writes to avoid blocking threads.

### Configuration
Ensure your `application.properties` is tuned for production:

```properties
# Enable TLS for security
eventstore.use-tls=true
# Add bearer-token auth if the server requires it
eventstore.auth-token=${EVENTSTORE_AUTH_TOKEN}
# Adjust timeout based on network latency (default 5000ms)
eventstore.timeout-ms=2000
```

### Constraints & Data Integrity
Use `@GraveyardField` constraints to describe schema metadata. The Java SDK includes a client-side `SchemaValidator` helper for preflight checks, but backend enforcement remains the source of truth. Non-nullable fields are exported as `required=true` in the generated schema so the schema model and annotations stay aligned. Regex validation is currently best treated as client-side validation unless your backend version explicitly enforces it.

### TLS and Auth
`eventstore.use-tls=true` enables gRPC transport security with the JVM's configured trust store. The default is plaintext for local development only. If you need a custom CA bundle or a more specialized auth flow, build and pass your own `ManagedChannel` to `EventStoreClient` instead of relying on the Spring-configured channel bean.
