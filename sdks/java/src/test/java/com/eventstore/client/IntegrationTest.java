package com.eventstore.client;

import com.eventstore.client.annotations.GraveyardEntity;
import com.eventstore.client.annotations.GraveyardField;
import com.eventstore.client.config.EventStoreConfig;
import com.eventstore.client.model.UpsertSchemaResponse;
import io.grpc.ManagedChannel;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Optional integration coverage for a live GraveyardDB instance.
 *
 * Set EVENTSTORE_INTEGRATION_TESTS=true to enable.
 * Optional overrides:
 * - EVENTSTORE_HOST
 * - EVENTSTORE_PORT
 * - EVENTSTORE_USE_TLS
 * - EVENTSTORE_AUTH_TOKEN
 * - EVENTSTORE_TIMEOUT_MS
 */
public class IntegrationTest {

    private static ManagedChannel channel;
    private static EventStoreClient client;

    @BeforeAll
    static void setUp() {
        Assumptions.assumeTrue(isIntegrationEnabled(), "Set EVENTSTORE_INTEGRATION_TESTS=true to run integration tests");

        EventStoreConfig config = loadConfig();
        channel = config.eventStoreChannel();
        client = new EventStoreClient(channel, config);
    }

    @AfterAll
    static void tearDown() {
        if (channel != null) {
            channel.shutdown();
        }
    }

    @Test
    void testUpsertSchema() {
        UpsertSchemaResponse response = client.upsertSchema(IntegrationUser.class);
        assertTrue(response.getSuccess(), "Schema upsert should succeed");
    }

    private static boolean isIntegrationEnabled() {
        String envValue = System.getenv("EVENTSTORE_INTEGRATION_TESTS");
        if (envValue != null) {
            return Boolean.parseBoolean(envValue);
        }

        String propertyValue = System.getProperty("eventstore.integrationTests");
        return propertyValue != null && Boolean.parseBoolean(propertyValue);
    }

    private static EventStoreConfig loadConfig() {
        EventStoreConfig config = new EventStoreConfig();
        config.setHost(System.getenv().getOrDefault("EVENTSTORE_HOST", "localhost"));
        config.setPort(parseInt(System.getenv().getOrDefault("EVENTSTORE_PORT", "50051"), 50051));
        config.setUseTls(Boolean.parseBoolean(System.getenv().getOrDefault("EVENTSTORE_USE_TLS", "false")));
        config.setAuthToken(System.getenv().getOrDefault("EVENTSTORE_AUTH_TOKEN", ""));
        config.setTimeoutMs(parseLong(System.getenv().getOrDefault("EVENTSTORE_TIMEOUT_MS", "5000"), 5000L));
        return config;
    }

    private static int parseInt(String value, int fallback) {
        try {
            return Integer.parseInt(value);
        } catch (NumberFormatException e) {
            return fallback;
        }
    }

    private static long parseLong(String value, long fallback) {
        try {
            return Long.parseLong(value);
        } catch (NumberFormatException e) {
            return fallback;
        }
    }

    @GraveyardEntity("integration_user")
    static class IntegrationUser {
        @GraveyardField(minLength = 3)
        String username;

        @GraveyardField(min = 18)
        int age;
    }
}
