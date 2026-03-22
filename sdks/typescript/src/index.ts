export { EventStoreClient, ANY_VERSION, normalizeExpectedVersion } from './client';
export type { EventStoreConfig } from './config';
export { defaultConfig } from './config';
export { GraveyardEntity, ENTITY_METADATA_KEY } from './decorators/entity';
export { GraveyardField, FIELD_METADATA_KEY } from './decorators/field';
export { SchemaGenerator } from './schema/generator';
export * from './proto/eventstore';
