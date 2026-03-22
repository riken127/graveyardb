package com.eventstore.client.config;

import io.grpc.CallOptions;
import io.grpc.Channel;
import io.grpc.ClientCall;
import io.grpc.Metadata;
import io.grpc.MethodDescriptor;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class EventStoreConfigTest {

    private static final Metadata.Key<String> AUTHORIZATION_HEADER =
            Metadata.Key.of("authorization", Metadata.ASCII_STRING_MARSHALLER);

    @Test
    void bearerTokenInterceptorAddsAuthorizationHeader() {
        BearerTokenInterceptor interceptor = new BearerTokenInterceptor("secret-token");
        RecordingClientCall recordingCall = new RecordingClientCall();
        Channel next = new Channel() {
            @Override
            public <ReqT, RespT> ClientCall<ReqT, RespT> newCall(
                    MethodDescriptor<ReqT, RespT> methodDescriptor,
                    CallOptions callOptions) {
                @SuppressWarnings("unchecked")
                ClientCall<ReqT, RespT> call = (ClientCall<ReqT, RespT>) recordingCall;
                return call;
            }

            @Override
            public String authority() {
                return "test-authority";
            }
        };

        ClientCall<Object, Object> call = interceptor.interceptCall(dummyMethod(), CallOptions.DEFAULT, next);
        call.start(new ClientCall.Listener<Object>() {}, new Metadata());

        assertEquals("Bearer secret-token", recordingCall.headers.get(AUTHORIZATION_HEADER));
    }

    private static MethodDescriptor<Object, Object> dummyMethod() {
        return MethodDescriptor.<Object, Object>newBuilder()
                .setType(MethodDescriptor.MethodType.UNARY)
                .setFullMethodName("eventstore.EventStore/Test")
                .setRequestMarshaller(new NoopMarshaller())
                .setResponseMarshaller(new NoopMarshaller())
                .build();
    }

    private static final class RecordingClientCall extends ClientCall<Object, Object> {
        private Metadata headers;

        @Override
        public void start(Listener<Object> responseListener, Metadata headers) {
            this.headers = headers;
        }

        @Override
        public void request(int numMessages) {
        }

        @Override
        public void cancel(String message, Throwable cause) {
        }

        @Override
        public void halfClose() {
        }

        @Override
        public void sendMessage(Object message) {
        }
    }

    private static final class NoopMarshaller implements MethodDescriptor.Marshaller<Object> {
        @Override
        public java.io.InputStream stream(Object value) {
            return java.io.InputStream.nullInputStream();
        }

        @Override
        public Object parse(java.io.InputStream stream) {
            return new Object();
        }
    }
}
