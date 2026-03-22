package com.eventstore.client.config;

import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

@Configuration
public class EventStoreConfig {

    @Value("${eventstore.host:localhost}")
    private String host = "localhost";

    @Value("${eventstore.port:50051}")
    private int port = 50051;

    @Value("${eventstore.use-tls:false}")
    private boolean useTls = false;

    @Value("${eventstore.timeout-ms:5000}")
    private long timeoutMs = 5000L;

    public String getHost() {
        return host;
    }

    public void setHost(String host) {
        this.host = host;
    }

    public int getPort() {
        return port;
    }

    public void setPort(int port) {
        this.port = port;
    }

    public boolean isUseTls() {
        return useTls;
    }

    public void setUseTls(boolean useTls) {
        this.useTls = useTls;
    }

    public long getTimeoutMs() {
        return timeoutMs;
    }

    public void setTimeoutMs(long timeoutMs) {
        this.timeoutMs = timeoutMs;
    }

    @Bean
    public ManagedChannel eventStoreChannel() {
        ManagedChannelBuilder<?> builder = ManagedChannelBuilder.forAddress(host, port);

        if (useTls) {
            builder.useTransportSecurity();
        } else {
            builder.usePlaintext();
        }

        return builder.build();
    }
}
