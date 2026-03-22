package com.eventstore.client.config;

import io.grpc.CallOptions;
import io.grpc.ClientCall;
import io.grpc.ClientInterceptor;
import io.grpc.ForwardingClientCall;
import io.grpc.Metadata;
import io.grpc.MethodDescriptor;
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

    /**
     * Bearer token attached to outgoing requests when set.
     * Leave blank for unauthenticated local development.
     */
    @Value("${eventstore.auth-token:}")
    private String authToken = "";

    /**
     * Default per-RPC timeout in milliseconds.
     * A finite timeout is recommended for all production deployments.
     */
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

    public String getAuthToken() {
        return authToken;
    }

    public void setAuthToken(String authToken) {
        this.authToken = authToken;
    }

    public long getTimeoutMs() {
        return timeoutMs;
    }

    public void setTimeoutMs(long timeoutMs) {
        this.timeoutMs = timeoutMs;
    }

    @Bean(destroyMethod = "shutdown")
    public ManagedChannel eventStoreChannel() {
        return createChannel();
    }

    ManagedChannel createChannel() {
        ManagedChannelBuilder<?> builder = ManagedChannelBuilder.forAddress(host, port);

        if (useTls) {
            builder.useTransportSecurity();
        } else {
            // Plaintext is intended for local development only.
            builder.usePlaintext();
        }

        if (authToken != null && !authToken.isBlank()) {
            builder.intercept(new BearerTokenInterceptor(authToken));
        }

        return builder.build();
    }
}

final class BearerTokenInterceptor implements ClientInterceptor {
    private static final Metadata.Key<String> AUTHORIZATION_HEADER =
            Metadata.Key.of("authorization", Metadata.ASCII_STRING_MARSHALLER);

    private final String token;

    BearerTokenInterceptor(String token) {
        this.token = token;
    }

    @Override
    public <ReqT, RespT> ClientCall<ReqT, RespT> interceptCall(
            MethodDescriptor<ReqT, RespT> method,
            CallOptions callOptions,
            io.grpc.Channel next) {
        ClientCall<ReqT, RespT> call = next.newCall(method, callOptions);
        return new ForwardingClientCall.SimpleForwardingClientCall<>(call) {
            @Override
            public void start(Listener<RespT> responseListener, Metadata headers) {
                headers.put(AUTHORIZATION_HEADER, "Bearer " + token);
                super.start(responseListener, headers);
            }
        };
    }
}
