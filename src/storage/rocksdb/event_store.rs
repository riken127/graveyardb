use rocksdb::{IteratorMode, Options, DB};
use tonic::async_trait;

use crate::domain::schema::model::Schema;
use crate::{
    domain::events::event::{Event, Transition},
    storage::event_store::{EventStore, EventStoreError},
};

use std::sync::Arc;
use tokio::sync::Mutex;

/// RocksDB-backed `EventStore` implementation.
pub struct RocksEventStore {
    db: DB,
    write_lock: Arc<Mutex<()>>,
}

impl RocksEventStore {
    /// Opens or creates a RocksDB event store at `path`.
    pub fn new(path: &str) -> Result<Self, EventStoreError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).map_err(|e| EventStoreError::StorageError(e.to_string()))?;
        Ok(Self {
            db,
            write_lock: Arc::new(Mutex::new(())),
        })
    }
}

#[async_trait]
impl EventStore for RocksEventStore {
    async fn append_event(
        &self,
        stream: &str,
        mut event: Event,
        expected_version: u64,
    ) -> Result<(), EventStoreError> {
        // Serialize writes so version checks and updates are atomic.
        let _guard = self.write_lock.lock().await;

        let meta_key = format!("meta:{}", stream);

        let current_version = match self
            .db
            .get(&meta_key)
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?
        {
            Some(v_bytes) => {
                let v_str = String::from_utf8_lossy(&v_bytes);
                v_str.parse::<u64>().unwrap_or(0)
            }
            None => 0,
        };

        if current_version != expected_version {
            return Err(EventStoreError::ConcurrencyError {
                expected: expected_version,
                actual: current_version,
            });
        }

        let next_version = current_version + 1;
        event.sequence_number = next_version;

        // Left-padded version keeps lexicographic ordering aligned with sequence order.
        let key = format!("stream:{}:{:020}", stream, next_version);

        let value = serde_cbor::to_vec(&event)?;

        let mut batch = rocksdb::WriteBatch::default();
        batch.put(key, value);
        batch.put(meta_key, next_version.to_string());

        self.db
            .write(batch)
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn fetch_stream(&self, stream: &str) -> Result<Vec<Event>, EventStoreError> {
        let prefix = format!("stream:{}:", stream);
        let mut events = Vec::new();
        let mode = IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward);

        for item in self.db.iterator(mode) {
            let (key, value) = item.map_err(|e| EventStoreError::StorageError(e.to_string()))?;
            if !key.starts_with(prefix.as_bytes()) {
                break;
            }

            let event: Event = serde_cbor::from_slice(&value)?;
            events.push(event);
        }
        Ok(events)
    }

    async fn upsert_schema(&self, schema: Schema) -> Result<(), EventStoreError> {
        // Persist schema history in the event log and update the latest projection.
        let key = format!("schema:{}", schema.name);

        let stream = format!("$schema:{}", schema.name);
        let events = self.fetch_stream(&stream).await?;
        let ver = events.last().map(|e| e.sequence_number).unwrap_or(0);

        let payload_bytes = serde_cbor::to_vec(&schema)?;
        let event = Event::new(
            &stream,
            crate::domain::events::event_kind::EventKind::Schematic,
            crate::domain::events::event_kind::EventPayload(payload_bytes.clone()),
            Transition::new("schema.upserted", "schema.previous", "schema.current"),
        );

        self.append_event(&stream, event, ver).await?;

        // Update projection
        self.db
            .put(key, payload_bytes)
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn get_schema(&self, name: &str) -> Result<Option<Schema>, EventStoreError> {
        let key = format!("schema:{}", name);
        match self
            .db
            .get(key)
            .map_err(|e| EventStoreError::StorageError(e.to_string()))?
        {
            Some(value) => {
                let schema = serde_cbor::from_slice(&value)?;
                Ok(Some(schema))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::event_kind::{EventKind, EventPayload};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_rocks_persistence() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().to_str().unwrap();

        let payload = EventPayload(vec![1, 2, 3]);
        let event = Event::new(
            "stream-p",
            EventKind::Internal,
            payload.clone(),
            Transition::new("event.appended", "pending", "persisted"),
        );

        {
            let store = RocksEventStore::new(db_path).expect("failed to open db");
            store
                .append_event("stream-p", event.clone(), 0)
                .await
                .expect("failed to append");
        } // store dropped, db closed

        {
            let store = RocksEventStore::new(db_path).expect("failed to reopen db");
            let loaded = store
                .fetch_stream("stream-p")
                .await
                .expect("failed to fetch");
            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0].id.0, event.id.0);
            assert_eq!(loaded[0].payload.0, payload.0);
        }
    }

    #[tokio::test]
    async fn test_stream_ordering() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().to_str().unwrap();
        let store = RocksEventStore::new(db_path).expect("failed to open db");

        let event1 = Event::new(
            "stream-o",
            EventKind::Internal,
            EventPayload(vec![1]),
            Transition::new("event.appended", "pending", "persisted"),
        );
        let event2 = Event::new(
            "stream-o",
            EventKind::Internal,
            EventPayload(vec![2]),
            Transition::new("event.appended", "pending", "persisted"),
        );

        store
            .append_event("stream-o", event1.clone(), 0)
            .await
            .expect("failed to append 1");
        store
            .append_event("stream-o", event2.clone(), 1)
            .await
            .expect("failed to append 2");

        let loaded = store
            .fetch_stream("stream-o")
            .await
            .expect("failed to fetch");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].sequence_number, 1);
        assert_eq!(loaded[1].sequence_number, 2);
    }

    #[tokio::test]
    async fn test_concurrency_check() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().to_str().unwrap();
        let store = RocksEventStore::new(db_path).expect("failed to open db");

        let event1 = Event::new(
            "stream-c",
            EventKind::Internal,
            EventPayload(vec![1]),
            Transition::new("event.appended", "pending", "persisted"),
        );
        let event2 = Event::new(
            "stream-c",
            EventKind::Internal,
            EventPayload(vec![2]),
            Transition::new("event.appended", "pending", "persisted"),
        );
        let event3 = Event::new(
            "stream-c",
            EventKind::Internal,
            EventPayload(vec![3]),
            Transition::new("event.appended", "pending", "persisted"),
        );

        store
            .append_event("stream-c", event1.clone(), 0)
            .await
            .expect("should work");

        store
            .append_event("stream-c", event2.clone(), 1)
            .await
            .expect("should work");

        let res = store.append_event("stream-c", event3.clone(), 1).await;

        match res {
            Err(EventStoreError::ConcurrencyError { expected, actual }) => {
                assert_eq!(expected, 1);
                assert_eq!(actual, 2);
            }
            _ => panic!("Expected ConcurrencyError, got {:?}", res),
        }
    }
}
