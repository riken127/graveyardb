package com.eventstore.client;

import com.google.common.util.concurrent.ListenableFuture;
import com.eventstore.client.model.AppendEventRequest;
import com.eventstore.client.model.AppendEventResponse;
import com.eventstore.client.model.Event;
import com.eventstore.client.model.EventStoreGrpc;
import com.eventstore.client.model.GetEventsRequest;
import com.eventstore.client.model.GetSchemaRequest;
import com.eventstore.client.model.GetSchemaResponse;
import com.eventstore.client.model.GetSnapshotRequest;
import com.eventstore.client.model.SaveSnapshotRequest;
import com.eventstore.client.model.Transition;
import com.eventstore.client.model.UpsertSchemaRequest;
import com.eventstore.client.model.UpsertSchemaResponse;
import com.eventstore.client.annotations.GraveyardEntity;
import com.eventstore.client.config.EventStoreConfig;
import com.eventstore.client.schema.SchemaGenerator;
import io.grpc.ManagedChannel;
import org.springframework.stereotype.Service;

import java.util.Iterator;
import java.util.List;
import java.util.Objects;
import java.util.concurrent.TimeUnit;

/**
 * Client for interacting with the Graveyar_DB EventStore gRPC API.
 * <p>
 * This client provides synchronous and asynchronous methods to:
 * <ul>
 *     <li>Append events to streams with Optimistic Concurrency Control (OCC).</li>
 *     <li>Read events from streams.</li>
 *     <li>Manage schemas for event validation.</li>
 *     <li>Store and retrieve snapshots for stream state.</li>
 * </ul>
 */
@Service
public class EventStoreClient {
    public static final long ANY_VERSION = -1L;

    private final EventStoreTransport transport;
    private final EventStoreConfig config;

    /**
     * Creates a new client instance.
     *
     * @param channel The gRPC managed channel to the Graveyar_DB server.
     * @param config  Configuration for timeouts and connection settings.
     */
    public EventStoreClient(ManagedChannel channel, EventStoreConfig config) {
        this(new GrpcEventStoreTransport(channel), config);
    }

    EventStoreClient(EventStoreTransport transport, EventStoreConfig config) {
        this.transport = Objects.requireNonNull(transport, "transport");
        this.config = Objects.requireNonNull(config, "config");
    }

    /**
     * Appends a batch of events to a stream without requesting optimistic concurrency checks.
     * This is equivalent to calling {@link #appendEvent(String, List, long)} with {@link #ANY_VERSION}.
     *
     * @param streamId The distinct ID of the stream (e.g., "order-123").
     * @param events   The list of {@link Event} objects to append.
     * @return {@code true} if the events were successfully appended; {@code false} otherwise.
     */
    public boolean appendEvent(String streamId, List<Event> events) {
        return appendEvent(streamId, events, ANY_VERSION);
    }

    /**
     * Appends a batch of events to a stream, enforcing Optimistic Concurrency Control (OCC).
     *
     * @param streamId        The distinct ID of the stream.
     * @param events          The list of {@link Event} objects to append.
     * @param expectedVersion The expected version of the stream prior to this append.
     *                        Use {@link #ANY_VERSION} (-1) to disable the check, or a non-negative version to
     *                        enforce optimistic concurrency control.
     *                        Any other negative value is rejected before the request is sent.
     *                        If the server's current version does not match, the append fails.
     * @return {@code true} if successful; {@code false} if a concurrency conflict or other error occurred.
     */
    public boolean appendEvent(String streamId, List<Event> events, long expectedVersion) {
        validateAppendEvents(events);
        long normalizedExpectedVersion = normalizeExpectedVersion(expectedVersion);
        AppendEventRequest request = AppendEventRequest.newBuilder()
                .setStreamId(streamId)
                .addAllEvents(events)
                .setExpectedVersion(normalizedExpectedVersion)
                .build();

        AppendEventResponse response = transport.appendEvent(request, config.getTimeoutMs());
        return response.getSuccess();
    }

    /**
     * Asynchronously appends events to a stream.
     *
     * @param streamId        The stream ID.
     * @param events          The events to append.
     * @param expectedVersion The expected version for OCC. Use {@link #ANY_VERSION} for "no check".
     * @return A {@link ListenableFuture} representing the pending response.
     */
    public ListenableFuture<AppendEventResponse> appendEventAsync(String streamId, List<Event> events, long expectedVersion) {
        validateAppendEvents(events);
        long normalizedExpectedVersion = normalizeExpectedVersion(expectedVersion);
        AppendEventRequest request = AppendEventRequest.newBuilder()
                .setStreamId(streamId)
                .addAllEvents(events)
                .setExpectedVersion(normalizedExpectedVersion)
                .build();

        return transport.appendEventAsync(request, config.getTimeoutMs());
    }

    /**
     * Reads events from a stream.
     * <p>
     * Returns an iterator that streams events from the server.
     *
     * @param streamId The stream ID to read from.
     * @return An {@link Iterator} of {@link Event}s.
     * @throws io.grpc.StatusRuntimeException If the stream is not found or communication fails.
     */
    public Iterator<Event> getEvents(String streamId) {
        GetEventsRequest request = GetEventsRequest.newBuilder()
                .setStreamId(streamId)
                .build();

        return transport.getEvents(request, config.getTimeoutMs());
    }

    /**
     * Retrieves a schema definition by name.
     *
     * @param name The schema name.
     * @return The schema response from the server, including whether it was found.
     */
    public GetSchemaResponse getSchema(String name) {
        GetSchemaRequest request = GetSchemaRequest.newBuilder()
                .setName(name)
                .build();

        return transport.getSchema(request, config.getTimeoutMs());
    }

    /**
     * Registers or updates a schema for the domain entity.
     * <p>
     * The provided class must be annotated with {@link GraveyardEntity}.
     * The schema is generated from declared instance fields. {@link com.eventstore.client.annotations.GraveyardField}
     * adds nullability and constraint metadata, while unannotated fields are exported as nullable and unconstrained.
     *
     * @param entityClass The Java class representing the entity.
     * @return The response from the server indicating success or failure.
     * @throws IllegalArgumentException If the class is missing the {@code @GraveyardEntity} annotation.
     */
    public UpsertSchemaResponse upsertSchema(Class<?> entityClass) {
        if (!entityClass.isAnnotationPresent(GraveyardEntity.class)) {
            throw new IllegalArgumentException("Class " + entityClass.getName() + " is not annotated with @GraveyardEntity");
        }

        com.eventstore.client.model.Schema schema = SchemaGenerator.generate(entityClass);

        UpsertSchemaRequest request = UpsertSchemaRequest.newBuilder()
                .setSchema(schema)
                .build();

        return transport.upsertSchema(request, config.getTimeoutMs());
    }

    /**
     * Saves a snapshot of a stream's state at a specific version.
     *
     * @param streamId  The stream ID.
     * @param version   The version of the stream this snapshot corresponds to.
     * @param payload   The serialized state payload (e.g., JSON bytes).
     * @param timestamp The timestamp of the snapshot.
     * @return {@code true} if the snapshot was successfully saved.
     */
    public boolean saveSnapshot(String streamId, long version, byte[] payload, long timestamp) {
        com.eventstore.client.model.Snapshot snapshot = com.eventstore.client.model.Snapshot.newBuilder()
                .setStreamId(streamId)
                .setVersion(version)
                .setPayload(com.google.protobuf.ByteString.copyFrom(payload))
                .setTimestamp(timestamp)
                .build();

        SaveSnapshotRequest request = SaveSnapshotRequest.newBuilder()
                .setSnapshot(snapshot)
                .build();

        return transport.saveSnapshot(request, config.getTimeoutMs());
    }

    /**
     * Retrieves the latest snapshot for a stream.
     *
     * @param streamId The stream ID.
     * @return The {@link com.eventstore.client.model.Snapshot} if found, or {@code null} if no snapshot exists.
     */
    public com.eventstore.client.model.Snapshot getSnapshot(String streamId) {
        GetSnapshotRequest request = GetSnapshotRequest.newBuilder()
                .setStreamId(streamId)
                .build();

        return transport.getSnapshot(request, config.getTimeoutMs());
    }

    private static long normalizeExpectedVersion(long expectedVersion) {
        if (expectedVersion == ANY_VERSION || expectedVersion >= 0) {
            return expectedVersion;
        }

        throw new IllegalArgumentException(
                "expectedVersion must be EventStoreClient.ANY_VERSION (-1) or a non-negative stream version");
    }

    private static void validateAppendEvents(List<Event> events) {
        Objects.requireNonNull(events, "events");

        for (int index = 0; index < events.size(); index++) {
            Event event = Objects.requireNonNull(events.get(index), "events[" + index + "]");
            if (!event.hasTransition()) {
                throw new IllegalArgumentException("events[" + index + "].transition is required");
            }
            validateTransition(event.getTransition(), index);
        }
    }

    private static void validateTransition(Transition transition, int eventIndex) {
        if (transition == null) {
            throw new IllegalArgumentException("events[" + eventIndex + "].transition is required");
        }
        String name = transition.getName() == null ? "" : transition.getName().trim();
        String fromState = transition.getFromState() == null ? "" : transition.getFromState().trim();
        String toState = transition.getToState() == null ? "" : transition.getToState().trim();

        if (name.isEmpty()) {
            throw new IllegalArgumentException("events[" + eventIndex + "].transition.name must be a non-empty string");
        }
        if (fromState.isEmpty()) {
            throw new IllegalArgumentException("events[" + eventIndex + "].transition.from_state must be a non-empty string");
        }
        if (toState.isEmpty()) {
            throw new IllegalArgumentException("events[" + eventIndex + "].transition.to_state must be a non-empty string");
        }
        if (fromState.equals(toState)) {
            throw new IllegalArgumentException("events[" + eventIndex + "].transition.from_state and to_state must be different");
        }
    }
}

interface EventStoreTransport {
    AppendEventResponse appendEvent(AppendEventRequest request, long timeoutMs);

    ListenableFuture<AppendEventResponse> appendEventAsync(AppendEventRequest request, long timeoutMs);

    Iterator<Event> getEvents(GetEventsRequest request, long timeoutMs);

    GetSchemaResponse getSchema(GetSchemaRequest request, long timeoutMs);

    UpsertSchemaResponse upsertSchema(UpsertSchemaRequest request, long timeoutMs);

    boolean saveSnapshot(SaveSnapshotRequest request, long timeoutMs);

    com.eventstore.client.model.Snapshot getSnapshot(GetSnapshotRequest request, long timeoutMs);
}

final class GrpcEventStoreTransport implements EventStoreTransport {
    private final EventStoreGrpc.EventStoreBlockingStub blockingStub;
    private final EventStoreGrpc.EventStoreFutureStub futureStub;

    GrpcEventStoreTransport(ManagedChannel channel) {
        this.blockingStub = EventStoreGrpc.newBlockingStub(channel);
        this.futureStub = EventStoreGrpc.newFutureStub(channel);
    }

    @Override
    public AppendEventResponse appendEvent(AppendEventRequest request, long timeoutMs) {
        return blockingStub
                .withDeadlineAfter(timeoutMs, TimeUnit.MILLISECONDS)
                .appendEvent(request);
    }

    @Override
    public ListenableFuture<AppendEventResponse> appendEventAsync(AppendEventRequest request, long timeoutMs) {
        return futureStub
                .withDeadlineAfter(timeoutMs, TimeUnit.MILLISECONDS)
                .appendEvent(request);
    }

    @Override
    public Iterator<Event> getEvents(GetEventsRequest request, long timeoutMs) {
        return blockingStub
                .withDeadlineAfter(timeoutMs, TimeUnit.MILLISECONDS)
                .getEvents(request);
    }

    @Override
    public GetSchemaResponse getSchema(GetSchemaRequest request, long timeoutMs) {
        return blockingStub
                .withDeadlineAfter(timeoutMs, TimeUnit.MILLISECONDS)
                .getSchema(request);
    }

    @Override
    public UpsertSchemaResponse upsertSchema(UpsertSchemaRequest request, long timeoutMs) {
        return blockingStub
                .withDeadlineAfter(timeoutMs, TimeUnit.MILLISECONDS)
                .upsertSchema(request);
    }

    @Override
    public boolean saveSnapshot(SaveSnapshotRequest request, long timeoutMs) {
        return blockingStub
                .withDeadlineAfter(timeoutMs, TimeUnit.MILLISECONDS)
                .saveSnapshot(request)
                .getSuccess();
    }

    @Override
    public com.eventstore.client.model.Snapshot getSnapshot(GetSnapshotRequest request, long timeoutMs) {
        com.eventstore.client.model.GetSnapshotResponse response = blockingStub
                .withDeadlineAfter(timeoutMs, TimeUnit.MILLISECONDS)
                .getSnapshot(request);

        return response.getFound() ? response.getSnapshot() : null;
    }
}
