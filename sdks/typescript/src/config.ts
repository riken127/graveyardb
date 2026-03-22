export interface EventStoreConfig {
    host: string;
    port: number;
    /**
     * Enable TLS for production deployments.
     *
     * When false, the client uses plaintext gRPC for local development only.
     */
    useTls: boolean;
    /**
     * Optional CA bundle for TLS connections.
     *
     * When provided, the SDK loads this file and uses it as the trusted root
     * certificate set. If omitted, grpc-js falls back to the platform trust
     * store.
     */
    tlsCaFile?: string;
    /**
     * Default per-request timeout in milliseconds.
     *
     * This applies when the caller does not supply a shorter deadline.
     */
    timeoutMs: number;
    /**
     * Optional bearer token added to unary and streaming calls via the
     * authorization header.
     */
    authToken?: string;
}

export const defaultConfig: EventStoreConfig = {
    host: 'localhost',
    port: 50051,
    useTls: false,
    tlsCaFile: undefined,
    timeoutMs: 5000
};
