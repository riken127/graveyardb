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
  timeoutMs: 2000,
  authToken: process.env.EVENTSTORE_TOKEN
});
```

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

### Register Schema

```typescript
await client.upsertSchema(UserProfile);
```

### Append Events

```typescript
const result = await client.appendEvent('stream-1', [
  { id: '1', eventType: 'Created', payload: Buffer.from('...'), timestamp: Date.now() }
], ANY_VERSION);
```

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
