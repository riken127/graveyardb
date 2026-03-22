use crate::domain::events::event::Event;
use crate::domain::schema::model::Schema;
use crate::storage::event_store::{EventStore, EventStoreError};
use std::sync::Arc;
use tonic::async_trait;
use tracing::warn;

/// Event store wrapper with best-effort primary/fallback failover.
pub struct HybridEventStore {
    primary: Arc<dyn EventStore>,
    fallback: Arc<dyn EventStore>,
}

impl HybridEventStore {
    /// Builds a hybrid store from primary and fallback backends.
    pub fn new(primary: Arc<dyn EventStore>, fallback: Arc<dyn EventStore>) -> Self {
        Self { primary, fallback }
    }
}

#[async_trait]
impl EventStore for HybridEventStore {
    async fn append_event(
        &self,
        stream: &str,
        event: Event,
        expected_version: u64,
    ) -> Result<(), EventStoreError> {
        match self
            .primary
            .append_event(stream, event.clone(), expected_version)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!(
                    "Primary Storage failed during append: {}. Falling back to Secondary.",
                    e
                );
                self.fallback
                    .append_event(stream, event, expected_version)
                    .await
            }
        }
    }

    async fn fetch_stream(&self, stream: &str) -> Result<Vec<Event>, EventStoreError> {
        match self.primary.fetch_stream(stream).await {
            Ok(events) => Ok(events),
            Err(e) => {
                warn!(
                    "Primary Storage failed during fetch: {}. Falling back to Secondary.",
                    e
                );
                self.fallback.fetch_stream(stream).await
            }
        }
    }

    async fn upsert_schema(&self, schema: Schema) -> Result<(), EventStoreError> {
        match self.primary.upsert_schema(schema.clone()).await {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!(
                    "Primary Storage failed during upsert_schema: {}. Falling back to Secondary.",
                    e
                );
                self.fallback.upsert_schema(schema).await
            }
        }
    }

    async fn get_schema(&self, name: &str) -> Result<Option<Schema>, EventStoreError> {
        match self.primary.get_schema(name).await {
            Ok(res) => Ok(res),
            Err(e) => {
                warn!(
                    "Primary Storage failed during get_schema: {}. Falling back to Secondary.",
                    e
                );
                self.fallback.get_schema(name).await
            }
        }
    }
}
