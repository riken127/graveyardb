use scylla::client::session::Session;
use scylla::client::session_builder::SessionBuilder;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ScyllaError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Query error: {0}")]
    QueryError(String),
}

pub struct ScyllaStore {
    session: Session,
    keyspace: String,
}

impl ScyllaStore {
    pub async fn new(uri: &str, keyspace: &str) -> Result<Self, ScyllaError> {
        let session = SessionBuilder::new()
            .known_node(uri)
            .connection_timeout(Duration::from_secs(5))
            .build()
            .await
            .map_err(|e| ScyllaError::ConnectionError(e.to_string()))?;

        let store = Self {
            session,
            keyspace: keyspace.to_string(),
        };

        store.init_schema().await?;

        Ok(store)
    }

    async fn init_schema(&self) -> Result<(), ScyllaError> {
        // Create keyspace
        let create_keyspace = format!(
            "CREATE KEYSPACE IF NOT EXISTS {} \
             WITH replication = {{'class': 'SimpleStrategy', 'replication_factor': 1}}",
            self.keyspace
        );
        self.session
            .query_unpaged(create_keyspace, &[])
            .await
            .map_err(|e| ScyllaError::QueryError(e.to_string()))?;

        // Create table
        // stream_id is partition key, id (uuid v7) is clustering key for time ordering
        let create_table = format!(
            "CREATE TABLE IF NOT EXISTS {}.events ( \
             stream_id text, \
             version bigint, \
             id uuid, \
             event_type text, \
             payload blob, \
             timestamp bigint, \
             metadata map<text, text>, \
             PRIMARY KEY (stream_id, version))",
            self.keyspace
        );
        let create_schemas_table = format!(
            "CREATE TABLE IF NOT EXISTS {}.schemas ( \
             name text PRIMARY KEY, \
             definition blob, \
             updated_at timestamp)",
            self.keyspace
        );

        self.session
            .query_unpaged(create_table, &[])
            .await
            .map_err(|e| ScyllaError::QueryError(e.to_string()))?;

        self.session
            .query_unpaged(create_schemas_table, &[])
            .await
            .map_err(|e| ScyllaError::QueryError(e.to_string()))?;

        // Migration: Attempt to add metadata column if missing
        let alter_table = format!(
            "ALTER TABLE {}.events ADD metadata map<text, text>",
            self.keyspace
        );
        let _ = self.session.query_unpaged(alter_table, &[]).await; // Ignore error if exists

        Ok(())
    }

    pub fn get_session(&self) -> &Session {
        &self.session
    }
}

use crate::domain::events::event::Event;
use crate::domain::events::event_kind::{EventKind, EventPayload};
use crate::domain::schema::model::Schema;
use crate::storage::event_store::{EventStore, EventStoreError};
use tonic::async_trait;

#[async_trait]
impl EventStore for ScyllaStore {
    async fn append_event(
        &self,
        stream: &str,
        mut event: Event,
        expected_version: u64,
    ) -> Result<(), EventStoreError> {
        // Prepare query with LWT
        let query = format!(
            "INSERT INTO {}.events (stream_id, version, id, event_type, payload, timestamp, metadata) VALUES (?, ?, ?, ?, ?, ?, ?) IF NOT EXISTS",
            self.keyspace
        );

        let next_version = expected_version + 1;
        event.sequence_number = next_version; // Assign atomic version

        let id = event.id.0;
        let event_type_str = event.event_type.to_string();
        let payload = event.payload.0;
        let timestamp = event.timestamp.0 as i64;
        let version = next_version as i64;
        let metadata = event.metadata;

        let result = self
            .session
            .query_unpaged(
                query,
                (
                    stream,
                    version,
                    id,
                    event_type_str,
                    payload,
                    timestamp,
                    metadata,
                ),
            )
            .await
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        // Check LWT result
        // Use into_rows_result to parse safely
        if let Ok(rows) = result.into_rows_result() {
            // Try to parse the first column as boolean [applied]
            // We use (bool,) tuple to match the row structure partially.
            // Scylla driver allows ignoring trailing columns.
            if let Ok(mut iter) = rows.rows::<(bool,)>() {
                if let Some(Ok((applied,))) = iter.next() {
                    if !applied {
                        return Err(EventStoreError::ConcurrencyError {
                            expected: expected_version,
                            actual: 0, // TODO: Parse actual version from subsequent columns if needed
                        });
                    }
                }
            }
        }

        Ok(())
    }

    async fn fetch_stream(&self, stream: &str) -> Result<Vec<Event>, EventStoreError> {
        let query = format!(
            "SELECT stream_id, version, id, event_type, payload, timestamp, metadata FROM {}.events WHERE stream_id = ? ORDER BY version ASC",
            self.keyspace
        );

        let query_result = self
            .session
            .query_unpaged(query, (stream,))
            .await
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        let rows_result = query_result
            .into_rows_result()
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        let rows = rows_result
            .rows::<(
                String,
                i64,
                uuid::Uuid,
                String,
                Vec<u8>,
                i64,
                Option<std::collections::HashMap<String, String>>,
            )>()
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        let mut events = Vec::new();

        for row in rows {
            let (_stream_id, version, id, event_type_str, payload, timestamp, metadata) =
                row.map_err(|e| EventStoreError::StorageError(e.to_string()))?;

            // Reconstruct Event
            let event_type =
                crate::domain::events::event_kind::EventKind::from_type_name(&event_type_str);

            events.push(Event {
                id: crate::domain::events::event_kind::EventId(id),
                stream_id: stream.to_string(), // we queried by this stream
                sequence_number: version as u64,
                event_type,
                payload: crate::domain::events::event_kind::EventPayload(payload),
                timestamp: crate::domain::events::event_kind::Timestamp(timestamp as u64),
                metadata: metadata.unwrap_or_default(),
            });
        }
        Ok(events)
    }

    async fn upsert_schema(&self, schema: Schema) -> Result<(), EventStoreError> {
        // 1. Append to Migration Log (Event Stream)
        let stream_id = format!("$schema:{}", schema.name);

        // Serialize Schema to Payload
        let payload_bytes = serde_cbor::to_vec(&schema)?;

        let migration_event = Event::new(
            &stream_id,
            EventKind::Schematic,
            EventPayload(payload_bytes.clone()),
        );

        // For schema stream, we might need relaxed concurrency or strictly ordered.
        // For now, let's assume schemas are rare and we can just try to append with version 0 if new, OR better,
        // Since schema migrations are rare, we can fetch head. But `EventPipeline` is not used here?
        // Wait, `upsert_schema` is in `EventStore`.
        // To do this strictly, we need to know the tail.
        // Let's FETCH first. (Slow, but safe for schema).

        let events = self.fetch_stream(&stream_id).await?;
        let next_version = events.last().map(|e| e.sequence_number).unwrap_or(0);

        self.append_event(&stream_id, migration_event, next_version)
            .await?;

        // 2. Update Current State Table
        let query = format!(
            "INSERT INTO {}.schemas (name, definition, updated_at) VALUES (?, ?, ?)",
            self.keyspace
        );

        // Use current timestamp for updated_at
        let updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap() // Safe unwrap for now
            .as_millis() as i64;

        self.session
            .query_unpaged(query, (schema.name, payload_bytes, updated_at))
            .await
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn get_schema(&self, name: &str) -> Result<Option<Schema>, EventStoreError> {
        let query = format!(
            "SELECT definition FROM {}.schemas WHERE name = ?",
            self.keyspace
        );

        let query_result = self
            .session
            .query_unpaged(query, (name,))
            .await
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        let rows_result = query_result
            .into_rows_result()
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        // Use standard iterator
        let mut rows = rows_result
            .rows::<(Vec<u8>,)>()
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        if let Some(row_res) = rows.next() {
            let (bytes,) = row_res.map_err(|e| EventStoreError::StorageError(e.to_string()))?;
            let schema: Schema = serde_cbor::from_slice(&bytes)?;
            return Ok(Some(schema));
        }

        Ok(None)
    }
}
