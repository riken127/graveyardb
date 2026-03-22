import * as grpc from '@grpc/grpc-js';
import { EventStoreClient as GrpcClient } from './proto/eventstore';
import { EventStoreConfig, defaultConfig } from './config';
import { Event, AppendEventRequest, GetEventsRequest, UpsertSchemaRequest, UpsertSchemaResponse, AppendEventResponse } from './proto/eventstore';
import { SchemaGenerator } from './schema/generator';

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

export class EventStoreClient {
    private client: GrpcClient;
    private config: EventStoreConfig;

    constructor(config: Partial<EventStoreConfig> = {}) {
        this.config = { ...defaultConfig, ...config };

        const address = `${this.config.host}:${this.config.port}`;
        const credentials = this.config.useTls
            ? grpc.credentials.createSsl()
            : grpc.credentials.createInsecure();

        this.client = new GrpcClient(address, credentials);
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

    close() {
        this.client.close();
    }
}
