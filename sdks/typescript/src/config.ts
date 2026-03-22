export interface EventStoreConfig {
    host: string;
    port: number;
    useTls: boolean;
    /**
     * Optional CA bundle for TLS connections.
     *
     * When provided, the SDK loads this file and uses it as the trusted root
     * certificate set. If omitted, grpc-js falls back to the platform trust
     * store.
     */
    tlsCaFile?: string;
    timeoutMs: number;
    authToken?: string;
}

export const defaultConfig: EventStoreConfig = {
    host: 'localhost',
    port: 50051,
    useTls: false,
    tlsCaFile: undefined,
    timeoutMs: 5000
};
