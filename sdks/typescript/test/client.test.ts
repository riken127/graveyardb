import { ANY_VERSION, EventStoreClient, GraveyardEntity, GraveyardField, SchemaGenerator, normalizeExpectedVersion } from '../src';

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
    // Note: To fully mock grpc-js in jest requires more setup or separate integration test
    // For now, we are verifying compilation and basic class structure.
    // In a real scenario, we would mock the GrpcClient prototype.

    let client: EventStoreClient;

    beforeEach(() => {
        client = new EventStoreClient({ host: 'localhost', port: 50051 });
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
});
