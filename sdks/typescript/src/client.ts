import * as grpc from '@grpc/grpc-js';
import { readFileSync } from 'fs';
import { EventStoreClient as GrpcClient } from './proto/eventstore';
import { EventStoreConfig, defaultConfig } from './config';
import {
    AppendEventRequest,
    AppendEventResponse,
    Event,
    GetEventsRequest,
    GetSchemaRequest,
    GetSchemaResponse,
    GetSnapshotRequest,
    GetSnapshotResponse,
    SaveSnapshotRequest,
    Snapshot,
    Transition,
    UpsertSchemaRequest,
    UpsertSchemaResponse
} from './proto/eventstore';
import { SchemaGenerator } from './schema/generator';

/**
 * Sentinel value that disables optimistic concurrency checks.
 *
 * Pass this constant when you want the server to append regardless of the
 * current stream version. Any non-negative integer requires an exact match.
 */
export const ANY_VERSION = -1 as const;

export function normalizeExpectedVersion(expectedVersion: number): number {
    if (!Number.isInteger(expectedVersion)) {
        throw new Error(`expectedVersion must be an integer, got ${expectedVersion}`);
    }

    if (expectedVersion < ANY_VERSION) {
        throw new Error(`expectedVersion must be ${ANY_VERSION} or a non-negative integer`);
    }

    if (!Number.isSafeInteger(expectedVersion)) {
        throw new Error(`expectedVersion must be a safe integer, got ${expectedVersion}`);
    }

    return expectedVersion;
}

type GrpcClientLike = Pick<
    GrpcClient,
    'appendEvent' | 'getEvents' | 'upsertSchema' | 'getSchema' | 'saveSnapshot' | 'getSnapshot' | 'close'
>;

function validateTransitionText(value: unknown, fieldName: string, eventIndex: number): string {
    if (typeof value !== 'string' || value.trim().length === 0) {
        throw new Error(`events[${eventIndex}].transition.${fieldName} must be a non-empty string`);
    }

    return value;
}

export function validateEventTransition(event: Event, eventIndex = 0): Transition {
    const transition = (event as Event & { transition?: Transition }).transition;

    if (!transition) {
        throw new Error(`events[${eventIndex}].transition is required`);
    }

    const fromState = validateTransitionText(transition.fromState, 'fromState', eventIndex);
    const toState = validateTransitionText(transition.toState, 'toState', eventIndex);
    if (fromState === toState) {
        throw new Error(`events[${eventIndex}].transition.fromState and toState must be different`);
    }

    return {
        name: validateTransitionText(transition.name, 'name', eventIndex),
        fromState,
        toState
    };
}

function validateAppendEvents(events: Event[]): void {
    events.forEach((event, index) => {
        if (!event) {
            throw new Error(`events[${index}] must not be empty`);
        }

        validateEventTransition(event, index);
    });
}

export class EventStoreClient {
    private client: GrpcClientLike;
    private config: EventStoreConfig;

    constructor(config: Partial<EventStoreConfig> = {}, client?: GrpcClientLike) {
        this.config = { ...defaultConfig, ...config };

        if (client) {
            this.client = client;
            return;
        }

        const address = `${this.config.host}:${this.config.port}`;
        const credentials = this.createCredentials();

        this.client = new GrpcClient(address, credentials);
    }

    private createCredentials(): grpc.ChannelCredentials {
        if (!this.config.useTls) {
            return grpc.credentials.createInsecure();
        }

        let rootCerts: Buffer | null = null;
        if (this.config.tlsCaFile) {
            try {
                rootCerts = readFileSync(this.config.tlsCaFile);
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                throw new Error(`Failed to read TLS CA file ${this.config.tlsCaFile}: ${message}`);
            }
        }

        return grpc.credentials.createSsl(rootCerts);
    }

    private getDeadline(): Date {
        return new Date(Date.now() + this.config.timeoutMs);
    }

    private buildMetadata(): grpc.Metadata {
        const metadata = new grpc.Metadata();

        if (this.config.authToken) {
            metadata.set('authorization', `Bearer ${this.config.authToken}`);
        }

        return metadata;
    }

    async appendEvent(streamId: string, events: Event[], expectedVersion: number = ANY_VERSION): Promise<boolean> {
        validateAppendEvents(events);

        const req: AppendEventRequest = {
            streamId,
            events,
            expectedVersion: normalizeExpectedVersion(expectedVersion),
            isForwarded: false
        };

        return new Promise((resolve, reject) => {
            const metadata = this.buildMetadata();
            this.client.appendEvent(req, metadata, { deadline: this.getDeadline() }, (err, response) => {
                if (err) return reject(err);
                if (!response) return reject(new Error('No response received'));
                resolve(response.success);
            });
        });
    }

    async getEvents(streamId: string): Promise<Event[]> {
        const req: GetEventsRequest = { streamId };
        const events: Event[] = [];

        return new Promise((resolve, reject) => {
            const stream = this.client.getEvents(req, this.buildMetadata(), { deadline: this.getDeadline() });

            stream.on('data', (event: Event) => {
                events.push(event);
            });

            stream.on('end', () => {
                resolve(events);
            });

            stream.on('error', (err) => {
                reject(err);
            });
        });
    }

    async upsertSchema(entityClass: Function): Promise<UpsertSchemaResponse> {
        const schema = SchemaGenerator.generate(entityClass);
        const req: UpsertSchemaRequest = { schema };

        return new Promise((resolve, reject) => {
            const metadata = this.buildMetadata();
            this.client.upsertSchema(req, metadata, { deadline: this.getDeadline() }, (err, response) => {
                if (err) return reject(err);
                if (!response) return reject(new Error('No response received'));
                resolve(response);
            });
        });
    }

    async getSchema(name: string): Promise<GetSchemaResponse> {
        const req: GetSchemaRequest = { name };

        return new Promise((resolve, reject) => {
            const metadata = this.buildMetadata();
            this.client.getSchema(req, metadata, { deadline: this.getDeadline() }, (err, response) => {
                if (err) return reject(err);
                if (!response) return reject(new Error('No response received'));
                resolve(response);
            });
        });
    }

    async saveSnapshot(streamId: string, version: number, payload: Buffer, timestamp: number): Promise<boolean> {
        const req: SaveSnapshotRequest = {
            snapshot: {
                streamId,
                version,
                payload,
                timestamp
            }
        };

        return new Promise((resolve, reject) => {
            const metadata = this.buildMetadata();
            this.client.saveSnapshot(req, metadata, { deadline: this.getDeadline() }, (err, response) => {
                if (err) return reject(err);
                if (!response) return reject(new Error('No response received'));
                resolve(response.success);
            });
        });
    }

    async getSnapshot(streamId: string): Promise<Snapshot | undefined> {
        const req: GetSnapshotRequest = { streamId };

        return new Promise((resolve, reject) => {
            const metadata = this.buildMetadata();
            this.client.getSnapshot(req, metadata, { deadline: this.getDeadline() }, (err, response) => {
                if (err) return reject(err);
                if (!response) return reject(new Error('No response received'));
                resolve(response.found ? response.snapshot : undefined);
            });
        });
    }

    close() {
        this.client.close();
    }
}
