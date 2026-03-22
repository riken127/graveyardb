# EventStore Java SDK

Java client library for `graveyar_db` using gRPC. Supports Spring Boot, async operations, and optimistic concurrency control.

## Features

- **Protocol**: gRPC with Protobuf.
- **Concurrency**: Optimistic locking via `expectedVersion`.
- **Resilience**: Configurable Timeouts.
- **Performance**: Async API (`ListenableFuture`) and Sync API.
- **Integration**: Spring Boot `@Service` and `@Configuration`.
- **Environment**: Easy toggle between Plaintext (Dev) and TLS (Prod).

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
| `eventstore.use-tls` | `false` | Set `true` for Production to check certificates. |
| `eventstore.timeout-ms` | `5000` | Timeout for requests in milliseconds. |

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
- `regex`: Regular expression pattern.
- `nullable`: Whether the field is optional (default: true).

Register the schema:

```java
client.upsertSchema(UserProfile.class);
```

### Append Sync

```java
List<Event> events = List.of(Event.newBuilder()...build());
// Use EventStoreClient.ANY_VERSION for "append regardless of current stream version".
boolean success = client.appendEvent("stream-1", events, EventStoreClient.ANY_VERSION);
```

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
# Adjust timeout based on network latency (default 5000ms)
eventstore.timeout-ms=2000
```

### Constraints & Data Integrity
Use `@GraveyardField` constraints to enforce schema validation at the definition level. Non-nullable fields are exported as `required=true` in the generated schema, which keeps the Java-side annotations aligned with validation behavior.
