package com.eventstore.client;

import com.eventstore.client.annotations.GraveyardEntity;
import com.eventstore.client.annotations.GraveyardField;
import com.eventstore.client.config.EventStoreConfig;
import com.eventstore.client.model.AppendEventRequest;
import com.eventstore.client.model.AppendEventResponse;
import com.eventstore.client.model.Event;
import com.eventstore.client.model.GetEventsRequest;
import com.eventstore.client.model.GetSnapshotRequest;
import com.eventstore.client.model.SaveSnapshotRequest;
import com.eventstore.client.model.Snapshot;
import com.eventstore.client.model.UpsertSchemaRequest;
import com.eventstore.client.model.UpsertSchemaResponse;
import com.google.common.util.concurrent.Futures;
import com.google.common.util.concurrent.ListenableFuture;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.util.Collections;
import java.util.Iterator;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

class EventStoreClientTest {

    private RecordingTransport transport;
    private EventStoreConfig eventStoreConfig;
    private EventStoreClient eventStoreClient;

    @BeforeEach
    void setUp() {
        transport = new RecordingTransport();
        eventStoreConfig = new EventStoreConfig();
        eventStoreConfig.setTimeoutMs(5000L);
        eventStoreClient = new EventStoreClient(transport, eventStoreConfig);
    }

    @Test
    void appendEvent_Success() {
        String streamId = "test-stream";
        List<Event> events = Collections.singletonList(Event.newBuilder().setId("1").build());
        long expectedVersion = 10L;

        transport.appendEventResponse = AppendEventResponse.newBuilder().setSuccess(true).build();

        boolean result = eventStoreClient.appendEvent(streamId, events, expectedVersion);

        assertTrue(result);
        assertNotNull(transport.lastAppendRequest);
        assertEquals(streamId, transport.lastAppendRequest.getStreamId());
        assertEquals(expectedVersion, transport.lastAppendRequest.getExpectedVersion());
        assertEquals(1, transport.lastAppendRequest.getEventsCount());
        assertEquals(5000L, transport.lastAppendTimeoutMs);
    }

    @Test
    void appendEvent_DefaultsToAnyVersion() {
        transport.appendEventResponse = AppendEventResponse.newBuilder().setSuccess(false).build();

        boolean result = eventStoreClient.appendEvent("stream", Collections.emptyList());

        assertFalse(result);
        assertNotNull(transport.lastAppendRequest);
        assertEquals(EventStoreClient.ANY_VERSION, transport.lastAppendRequest.getExpectedVersion());
        assertEquals(5000L, transport.lastAppendTimeoutMs);
    }

    @Test
    void appendEvent_RejectsUnsupportedNegativeVersion() {
        assertThrows(IllegalArgumentException.class,
                () -> eventStoreClient.appendEvent("stream", Collections.emptyList(), -2L));
    }

    @Test
    void getEvents_Success() {
        String streamId = "read-stream";
        Iterator<Event> mockIterator = Collections.<Event>emptyList().iterator();
        transport.getEventsResponse = mockIterator;

        Iterator<Event> result = eventStoreClient.getEvents(streamId);

        assertNotNull(result);
        assertSame(mockIterator, result);
        assertNotNull(transport.lastGetEventsRequest);
        assertEquals(streamId, transport.lastGetEventsRequest.getStreamId());
        assertEquals(5000L, transport.lastGetEventsTimeoutMs);
    }

    @Test
    void appendEventAsync_Success() {
        List<Event> events = Collections.emptyList();
        AppendEventResponse response = AppendEventResponse.newBuilder().setSuccess(true).build();
        transport.appendEventAsyncResponse = Futures.immediateFuture(response);

        ListenableFuture<AppendEventResponse> result = eventStoreClient.appendEventAsync("async-stream", events, -1);

        assertNotNull(result);
        assertNotNull(transport.lastAppendAsyncRequest);
        assertEquals(EventStoreClient.ANY_VERSION, transport.lastAppendAsyncRequest.getExpectedVersion());
        assertEquals(5000L, transport.lastAppendAsyncTimeoutMs);
    }

    @Test
    void upsertSchema_Success() {
        transport.upsertSchemaResponse = UpsertSchemaResponse.newBuilder().setSuccess(true).build();

        UpsertSchemaResponse result = eventStoreClient.upsertSchema(TestEntity.class);

        assertTrue(result.getSuccess());
        assertNotNull(transport.lastUpsertSchemaRequest);
        assertEquals(5000L, transport.lastUpsertSchemaTimeoutMs);

        com.eventstore.client.model.Schema schema = transport.lastUpsertSchemaRequest.getSchema();
        assertEquals("test_entity", schema.getName());
        assertTrue(schema.getFieldsMap().containsKey("name"));
        assertTrue(schema.getFieldsMap().containsKey("age"));
        assertTrue(schema.getFieldsMap().get("name").hasConstraints());
        assertTrue(schema.getFieldsMap().get("name").getConstraints().getRequired());
    }

    @Test
    void upsertSchema_WithConstraints() {
        transport.upsertSchemaResponse = UpsertSchemaResponse.newBuilder().setSuccess(true).build();

        UpsertSchemaResponse result = eventStoreClient.upsertSchema(ConstrainedEntity.class);

        assertTrue(result.getSuccess());
        assertNotNull(transport.lastUpsertSchemaRequest);
        assertEquals(5000L, transport.lastUpsertSchemaTimeoutMs);

        com.eventstore.client.model.Schema schema = transport.lastUpsertSchemaRequest.getSchema();
        assertTrue(schema.getFieldsMap().containsKey("age"));

        com.eventstore.client.model.Field ageField = schema.getFieldsMap().get("age");
        assertTrue(ageField.hasConstraints());
        assertEquals(0.0, ageField.getConstraints().getMinValue());
        assertEquals(150.0, ageField.getConstraints().getMaxValue());

        com.eventstore.client.model.Field usernameField = schema.getFieldsMap().get("username");
        assertTrue(usernameField.hasConstraints());
        assertEquals(3, usernameField.getConstraints().getMinLength());
        assertEquals("^[a-z]+$", usernameField.getConstraints().getRegex());
    }

    @Test
    void saveSnapshot_Success() {
        transport.saveSnapshotResponse = true;

        boolean saved = eventStoreClient.saveSnapshot("snapshot-stream", 3L, new byte[] {1, 2, 3}, 1234L);

        assertTrue(saved);
        assertNotNull(transport.lastSaveSnapshotRequest);
        assertEquals(5000L, transport.lastSaveSnapshotTimeoutMs);
        assertEquals("snapshot-stream", transport.lastSaveSnapshotRequest.getSnapshot().getStreamId());
        assertEquals(3L, transport.lastSaveSnapshotRequest.getSnapshot().getVersion());
    }

    @Test
    void getSnapshot_Success() {
        Snapshot snapshot = Snapshot.newBuilder()
                .setStreamId("snapshot-stream")
                .setVersion(7L)
                .setTimestamp(99L)
                .build();

        transport.getSnapshotResponse = snapshot;

        Snapshot result = eventStoreClient.getSnapshot("snapshot-stream");

        assertNotNull(result);
        assertEquals("snapshot-stream", result.getStreamId());
        assertEquals(7L, result.getVersion());
        assertNotNull(transport.lastGetSnapshotRequest);
        assertEquals(5000L, transport.lastGetSnapshotTimeoutMs);
    }

    private static final class RecordingTransport implements EventStoreTransport {
        private AppendEventRequest lastAppendRequest;
        private long lastAppendTimeoutMs;
        private AppendEventResponse appendEventResponse = AppendEventResponse.newBuilder().setSuccess(true).build();

        private AppendEventRequest lastAppendAsyncRequest;
        private long lastAppendAsyncTimeoutMs;
        private ListenableFuture<AppendEventResponse> appendEventAsyncResponse =
                Futures.immediateFuture(AppendEventResponse.newBuilder().setSuccess(true).build());

        private GetEventsRequest lastGetEventsRequest;
        private long lastGetEventsTimeoutMs;
        private Iterator<Event> getEventsResponse = Collections.emptyIterator();

        private UpsertSchemaRequest lastUpsertSchemaRequest;
        private long lastUpsertSchemaTimeoutMs;
        private UpsertSchemaResponse upsertSchemaResponse = UpsertSchemaResponse.newBuilder().setSuccess(true).build();

        private SaveSnapshotRequest lastSaveSnapshotRequest;
        private long lastSaveSnapshotTimeoutMs;
        private boolean saveSnapshotResponse = true;

        private GetSnapshotRequest lastGetSnapshotRequest;
        private long lastGetSnapshotTimeoutMs;
        private Snapshot getSnapshotResponse;

        @Override
        public AppendEventResponse appendEvent(AppendEventRequest request, long timeoutMs) {
            this.lastAppendRequest = request;
            this.lastAppendTimeoutMs = timeoutMs;
            return appendEventResponse;
        }

        @Override
        public ListenableFuture<AppendEventResponse> appendEventAsync(AppendEventRequest request, long timeoutMs) {
            this.lastAppendAsyncRequest = request;
            this.lastAppendAsyncTimeoutMs = timeoutMs;
            return appendEventAsyncResponse;
        }

        @Override
        public Iterator<Event> getEvents(GetEventsRequest request, long timeoutMs) {
            this.lastGetEventsRequest = request;
            this.lastGetEventsTimeoutMs = timeoutMs;
            return getEventsResponse;
        }

        @Override
        public UpsertSchemaResponse upsertSchema(UpsertSchemaRequest request, long timeoutMs) {
            this.lastUpsertSchemaRequest = request;
            this.lastUpsertSchemaTimeoutMs = timeoutMs;
            return upsertSchemaResponse;
        }

        @Override
        public boolean saveSnapshot(SaveSnapshotRequest request, long timeoutMs) {
            this.lastSaveSnapshotRequest = request;
            this.lastSaveSnapshotTimeoutMs = timeoutMs;
            return saveSnapshotResponse;
        }

        @Override
        public Snapshot getSnapshot(GetSnapshotRequest request, long timeoutMs) {
            this.lastGetSnapshotRequest = request;
            this.lastGetSnapshotTimeoutMs = timeoutMs;
            return getSnapshotResponse;
        }
    }

    @GraveyardEntity("test_entity")
    static class TestEntity {
        @GraveyardField(nullable = false)
        String name;

        int age;
    }

    @GraveyardEntity("constrained_entity")
    static class ConstrainedEntity {
        @GraveyardField(min = 0, max = 150)
        int age;

        @GraveyardField(minLength = 3, regex = "^[a-z]+$")
        String username;
    }
}
