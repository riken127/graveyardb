# EventStore TypeScript SDK

TypeScript client for `graveyard_db`. Supports gRPC, decorator-based schemas, and async/await.

## Features

- `gRPC` over HTTP/2 via `@grpc/grpc-js`
- Decorator-based schemas via `@GraveyardEntity` and `@GraveyardField`
- Schema constraints for min/max, length, and regex validation
- TLS, timeout, and bearer-token auth support

## Installation

```bash
npm install @eventstore/client
```

## Configuration

```typescript
import { ANY_VERSION, EventStoreClient } from '@eventstore/client';

const client = new EventStoreClient({
  host: 'localhost',
  port: 50051,
  useTls: process.env.NODE_ENV === 'production',
  tlsCaFile: process.env.EVENTSTORE_CA_FILE,
  timeoutMs: 2000,
  authToken: process.env.EVENTSTORE_TOKEN
});
```

`ANY_VERSION` is the sentinel `-1` for optimistic concurrency bypass. Pass a
non-negative integer when you want the server to enforce an exact version match.

## Usage

### Define Entities

```typescript
import { GraveyardEntity, GraveyardField } from '@eventstore/client';

@GraveyardEntity('user_profile')
class UserProfile {
  @GraveyardField({ minLength: 3, regex: '^[a-z]+$' })
  username: string;

  @GraveyardField({ min: 18, max: 150, nullable: false })
  age: number;
}
```

The schema generator currently infers primitives from decorator metadata and
supports nested decorated classes. Arrays, generics, and other erased runtime
types are rejected so the SDK does not silently publish the wrong schema.

### Register Schema

```typescript
await client.upsertSchema(UserProfile);
```

### Append Events

```typescript
const result = await client.appendEvent('stream-1', [
  {
    id: '1',
    eventType: 'Created',
    payload: Buffer.from('...'),
    timestamp: Date.now(),
    transition: { name: 'Created', fromState: 'draft', toState: 'published' }
  }
], ANY_VERSION);
```

If you need optimistic concurrency, replace `ANY_VERSION` with the current
stream version returned by your read path.

Every appended event must include `transition.name`, `transition.fromState`,
and `transition.toState`. The SDK also requires `fromState !== toState`.

## Public Imports

Prefer importing from the package root:

```typescript
import {
  ANY_VERSION,
  EventStoreClient,
  GraveyardEntity,
  GraveyardField,
  SchemaGenerator
} from '@eventstore/client';
```

## Development

```bash
npm install
npm run proto:gen
npm test
npm run build
```

Generated files live in `dist/` and should be treated as build output.

## TLS Notes

- Set `useTls: true` for production deployments.
- If the server uses a private CA, point `tlsCaFile` at the PEM bundle to trust
  that certificate chain.
- `authToken` is sent as `authorization: Bearer <token>` on outgoing requests.
