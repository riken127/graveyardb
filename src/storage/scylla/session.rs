use scylla::client::session::Session;
use scylla::client::session_builder::SessionBuilder;
use std::time::Duration;

/// Errors returned by Scylla store initialization and schema bootstrap.
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
    /// Creates a Scylla-backed store and ensures required tables exist.
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
        let create_keyspace = format!(
            "CREATE KEYSPACE IF NOT EXISTS {} \
             WITH replication = {{'class': 'SimpleStrategy', 'replication_factor': 1}}",
            self.keyspace
        );
        self.session
            .query_unpaged(create_keyspace, &[])
            .await
            .map_err(|e| ScyllaError::QueryError(e.to_string()))?;

        // `stream_id` is the partition key and `version` is the clustering key.
        let create_table = format!(
            "CREATE TABLE IF NOT EXISTS {}.events ( \
             stream_id text, \
             version bigint, \
             id uuid, \
             event_type text, \
             payload blob, \
             timestamp bigint, \
             metadata map<text, text>, \
             transition_name text, \
             transition_from_state text, \
             transition_to_state text, \
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

        // Backward-compatible migration for deployments created before `metadata`.
        let alter_table = format!(
            "ALTER TABLE {}.events ADD metadata map<text, text>",
            self.keyspace
        );
        let _ = self.session.query_unpaged(alter_table, &[]).await;

        let alter_transition_name = format!(
            "ALTER TABLE {}.events ADD transition_name text",
            self.keyspace
        );
        let _ = self.session.query_unpaged(alter_transition_name, &[]).await;

        let alter_transition_from_state = format!(
            "ALTER TABLE {}.events ADD transition_from_state text",
            self.keyspace
        );
        let _ = self
            .session
            .query_unpaged(alter_transition_from_state, &[])
            .await;

        let alter_transition_to_state = format!(
            "ALTER TABLE {}.events ADD transition_to_state text",
            self.keyspace
        );
        let _ = self
            .session
            .query_unpaged(alter_transition_to_state, &[])
            .await;

        Ok(())
    }

    pub fn get_session(&self) -> &Session {
        &self.session
    }

    async fn fetch_current_version(&self, stream: &str) -> Result<u64, EventStoreError> {
        let query = format!(
            "SELECT version FROM {}.events WHERE stream_id = ? ORDER BY version DESC LIMIT 1",
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

        let mut rows = rows_result
            .rows::<(i64,)>()
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        match rows.next() {
            Some(row) => {
                let (version,) = row.map_err(|e| EventStoreError::StorageError(e.to_string()))?;
                Ok(version as u64)
            }
            None => Ok(0),
        }
    }
}

use crate::domain::events::event::{Event, Transition};
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
        let current_version = self.fetch_current_version(stream).await?;
        if current_version != expected_version {
            return Err(EventStoreError::ConcurrencyError {
                expected: expected_version,
                actual: current_version,
            });
        }

        let query = format!(
            "INSERT INTO {}.events (stream_id, version, id, event_type, payload, timestamp, metadata, transition_name, transition_from_state, transition_to_state) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) IF NOT EXISTS",
            self.keyspace
        );

        let next_version = expected_version + 1;
        event.sequence_number = next_version;

        let id = event.id.0;
        let event_type_str = event.event_type.to_string();
        let payload = event.payload.0;
        let timestamp = event.timestamp.0 as i64;
        let version = next_version as i64;
        let metadata = event.metadata;
        let transition_name = event.transition.name;
        let transition_from_state = event.transition.from_state;
        let transition_to_state = event.transition.to_state;

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
                    transition_name,
                    transition_from_state,
                    transition_to_state,
                ),
            )
            .await
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        // Parse LWT outcome from the `[applied]` column.
        if let Ok(rows) = result.into_rows_result() {
            if let Ok(mut iter) = rows.rows::<(bool,)>() {
                if let Some(Ok((applied,))) = iter.next() {
                    if !applied {
                        return Err(EventStoreError::ConcurrencyError {
                            expected: expected_version,
                            // Current version is not decoded from the row projection.
                            actual: 0,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    async fn fetch_stream(&self, stream: &str) -> Result<Vec<Event>, EventStoreError> {
        let query = format!(
            "SELECT stream_id, version, id, event_type, payload, timestamp, metadata, transition_name, transition_from_state, transition_to_state FROM {}.events WHERE stream_id = ? ORDER BY version ASC",
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
                Option<String>,
                Option<String>,
                Option<String>,
            )>()
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        let mut events = Vec::new();

        for row in rows {
            let (
                _stream_id,
                version,
                id,
                event_type_str,
                payload,
                timestamp,
                metadata,
                transition_name,
                transition_from_state,
                transition_to_state,
            ) = row.map_err(|e| EventStoreError::StorageError(e.to_string()))?;

            let event_type =
                crate::domain::events::event_kind::EventKind::from_type_name(&event_type_str);
            let metadata = metadata.unwrap_or_default();
            let transition = Transition::new(
                transition_name
                    .or_else(|| metadata.get("transition.name").cloned())
                    .unwrap_or_else(|| "legacy.unknown".to_string()),
                transition_from_state
                    .or_else(|| metadata.get("transition.from_state").cloned())
                    .unwrap_or_else(|| "unknown".to_string()),
                transition_to_state
                    .or_else(|| metadata.get("transition.to_state").cloned())
                    .unwrap_or_else(|| "unknown".to_string()),
            );

            events.push(Event {
                id: crate::domain::events::event_kind::EventId(id),
                stream_id: stream.to_string(),
                sequence_number: version as u64,
                event_type,
                payload: crate::domain::events::event_kind::EventPayload(payload),
                timestamp: crate::domain::events::event_kind::Timestamp(timestamp as u64),
                transition,
                metadata,
            });
        }
        Ok(events)
    }

    async fn upsert_schema(&self, schema: Schema) -> Result<(), EventStoreError> {
        // Append to schema history stream.
        let stream_id = format!("$schema:{}", schema.name);
        let payload_bytes = serde_cbor::to_vec(&schema)?;

        let migration_event = Event::new(
            &stream_id,
            EventKind::Schematic,
            EventPayload(payload_bytes.clone()),
            Transition::new("schema.upserted", "schema.previous", "schema.current"),
        );

        // Fetch stream tail to preserve OCC guarantees for schema updates.
        let events = self.fetch_stream(&stream_id).await?;
        let next_version = events.last().map(|e| e.sequence_number).unwrap_or(0);

        self.append_event(&stream_id, migration_event, next_version)
            .await?;

        // Update current schema projection.
        let query = format!(
            "INSERT INTO {}.schemas (name, definition, updated_at) VALUES (?, ?, ?)",
            self.keyspace
        );

        let updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?
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

#[cfg(test)]
mod tests {
    #[test]
    fn exact_expected_version_check_is_strict() {
        fn validate(
            current_version: u64,
            expected_version: u64,
        ) -> Result<(), super::EventStoreError> {
            if current_version != expected_version {
                return Err(super::EventStoreError::ConcurrencyError {
                    expected: expected_version,
                    actual: current_version,
                });
            }

            Ok(())
        }

        assert!(validate(3, 3).is_ok());
        assert!(matches!(
            validate(3, 1),
            Err(super::EventStoreError::ConcurrencyError {
                expected: 1,
                actual: 3
            })
        ));
        assert!(matches!(
            validate(1, 3),
            Err(super::EventStoreError::ConcurrencyError {
                expected: 3,
                actual: 1
            })
        ));
    }
}
