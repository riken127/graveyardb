import * as grpc from '@grpc/grpc-js';
import { ANY_VERSION, EventStoreClient, GraveyardEntity, GraveyardField, SchemaGenerator, normalizeExpectedVersion, validateEventTransition } from '../src';

@GraveyardEntity("profile_details")
class ProfileDetails {
    @GraveyardField({ minLength: 2 })
    city!: string;
}

@GraveyardEntity("user_test")
class UserTest {
    @GraveyardField({ minLength: 3 })
    username!: string;

    @GraveyardField({ min: 18 })
    age!: number;

    @GraveyardField()
    profile!: ProfileDetails;
}

@GraveyardEntity("array_user")
class ArrayUser {
    @GraveyardField()
    tags!: string[];
}

describe('SchemaGenerator', () => {
    it('should generate schema with constraints', () => {
        const schema = SchemaGenerator.generate(UserTest);
        expect(schema.name).toBe("user_test");
        expect(schema.fields['username']).toBeDefined();
        expect(schema.fields['username'].constraints?.minLength).toBe(3);
        expect(schema.fields['age']).toBeDefined();
        expect(schema.fields['age'].constraints?.minValue).toBe(18);
        expect(schema.fields['profile'].fieldType?.subSchema?.name).toBe("profile_details");
        expect(schema.fields['profile'].fieldType?.subSchema?.fields['city']).toBeDefined();
    });

    it('should reject array fields until the generator can infer their element type', () => {
        expect(() => SchemaGenerator.generate(ArrayUser)).toThrow(/array/i);
    });
});

describe('EventStoreClient', () => {
    let client: EventStoreClient;
    let grpcClient: {
        appendEvent: jest.Mock;
        getEvents: jest.Mock;
        upsertSchema: jest.Mock;
        getSchema: jest.Mock;
        saveSnapshot: jest.Mock;
        getSnapshot: jest.Mock;
        close: jest.Mock;
    };

    beforeEach(() => {
        grpcClient = {
            appendEvent: jest.fn((req: { streamId: string }, metadata: grpc.Metadata, options: grpc.CallOptions, callback: (err: Error | null, response?: { success: boolean }) => void) => callback(null, { success: true })),
            getEvents: jest.fn(() => ({ on: jest.fn() })),
            upsertSchema: jest.fn((req: { schema?: unknown }, metadata: grpc.Metadata, options: grpc.CallOptions, callback: (err: Error | null, response?: { success: boolean; message: string }) => void) => callback(null, { success: true, message: 'ok' })),
            getSchema: jest.fn((req: { name: string }, metadata: grpc.Metadata, options: grpc.CallOptions, callback: (err: Error | null, response?: { found: boolean; schema?: { name: string } }) => void) => callback(null, { found: true, schema: { name: req.name } })),
            saveSnapshot: jest.fn((req: { snapshot?: { streamId: string; version: number; payload: Buffer; timestamp: number } }, metadata: grpc.Metadata, options: grpc.CallOptions, callback: (err: Error | null, response?: { success: boolean }) => void) => callback(null, { success: true })),
            getSnapshot: jest.fn((req: { streamId: string }, metadata: grpc.Metadata, options: grpc.CallOptions, callback: (err: Error | null, response?: { found: boolean; snapshot?: { streamId: string; version: number; payload: Buffer; timestamp: number } }) => void) => callback(null, { found: true, snapshot: { streamId: req.streamId, version: 9, payload: Buffer.from('state'), timestamp: 123 } })),
            close: jest.fn()
        };

        client = new EventStoreClient(
            { host: 'localhost', port: 50051, timeoutMs: 2500, authToken: 'secret-token' },
            grpcClient as any
        );
    });

    afterEach(() => {
        client.close();
    });

    it('should instantiate', () => {
        expect(client).toBeDefined();
    });

    it('should normalize expected version sentinel and validate bounds', () => {
        expect(normalizeExpectedVersion(ANY_VERSION)).toBe(ANY_VERSION);
        expect(normalizeExpectedVersion(4)).toBe(4);
        expect(() => normalizeExpectedVersion(-2)).toThrow();
        expect(() => normalizeExpectedVersion(1.5)).toThrow();
    });

    it('should validate transition presence and non-empty fields', () => {
        expect(() => validateEventTransition({
            id: '1',
            eventType: 'Created',
            payload: Buffer.from('{}'),
            timestamp: Date.now(),
            transition: { name: 'Activated', fromState: 'pending', toState: 'active' },
            metadata: {}
        })).not.toThrow();

        expect(() => validateEventTransition({
            id: '1',
            eventType: 'Created',
            payload: Buffer.from('{}'),
            timestamp: Date.now(),
            metadata: {}
        } as never)).toThrow(/transition is required/);

        expect(() => validateEventTransition({
            id: '1',
            eventType: 'Created',
            payload: Buffer.from('{}'),
            timestamp: Date.now(),
            transition: { name: ' ', fromState: 'pending', toState: 'active' },
            metadata: {}
        } as never)).toThrow(/transition\.name/);

        expect(() => validateEventTransition({
            id: '1',
            eventType: 'Created',
            payload: Buffer.from('{}'),
            timestamp: Date.now(),
            transition: { name: 'Activated', fromState: 'active', toState: 'active' },
            metadata: {}
        } as never)).toThrow(/must be different/);
    });

    it('should call getSchema with auth metadata and a deadline', async () => {
        const response = await client.getSchema('user_test');

        expect(response.found).toBe(true);
        expect(grpcClient.getSchema).toHaveBeenCalledTimes(1);

        const [request, metadata, options] = grpcClient.getSchema.mock.calls[0];
        expect(request).toEqual({ name: 'user_test' });
        expect((metadata as grpc.Metadata).get('authorization')).toEqual(['Bearer secret-token']);
        expect(options.deadline).toBeInstanceOf(Date);
    });

    it('should save snapshots and return the grpc success flag', async () => {
        const saved = await client.saveSnapshot('stream-1', 7, Buffer.from('payload'), 1234);

        expect(saved).toBe(true);
        expect(grpcClient.saveSnapshot).toHaveBeenCalledTimes(1);

        const [request] = grpcClient.saveSnapshot.mock.calls[0];
        expect(request).toEqual({
            snapshot: {
                streamId: 'stream-1',
                version: 7,
                payload: Buffer.from('payload'),
                timestamp: 1234
            }
        });
    });

    it('should return snapshots when found and undefined when missing', async () => {
        const snapshot = await client.getSnapshot('stream-1');
        expect(snapshot).toEqual({
            streamId: 'stream-1',
            version: 9,
            payload: Buffer.from('state'),
            timestamp: 123
        });

        grpcClient.getSnapshot.mockImplementationOnce((req: { streamId: string }, metadata: grpc.Metadata, options: grpc.CallOptions, callback: (err: Error | null, response?: { found: boolean }) => void) => callback(null, { found: false }));
        await expect(client.getSnapshot('missing-stream')).resolves.toBeUndefined();
    });
});
