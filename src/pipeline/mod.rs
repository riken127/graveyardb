pub mod command;
pub mod worker;

use crate::cluster::client::ClusterClient;
use crate::cluster::ClusterTopology;
use crate::domain::events::event::Event;
use crate::pipeline::command::PipelineCommand;
use crate::pipeline::worker::Worker;
use crate::storage::event_store::EventStore;
use crate::storage::event_store::EventStoreError;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

const NUM_WORKERS: usize = 32;

/// The primary event processing pipeline.
///
/// `EventPipeline` is responsibility for:
/// 1. Routing requests to the correct node (Owner) in the cluster/shard.
/// 2. Validating events against schemas (Optional).
/// 3. Serializing write requests per stream to ensure linearizability via workers.
/// 4. Delegating persistence to the `EventStore`.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("invalid expected version {0}; use -1 or a non-negative version")]
    InvalidExpectedVersion(i64),
    #[error("stream {stream_id} is owned by {owner}, not {current_node} (epoch {epoch})")]
    NotOwner {
        stream_id: String,
        owner: String,
        current_node: String,
        epoch: u64,
    },
    #[error("concurrency conflict: expected {expected}, actual {actual}")]
    Concurrency { expected: u64, actual: u64 },
    #[error("forwarding to {target} failed: {reason}")]
    Forwarding { target: String, reason: String },
    #[error("storage error: {0}")]
    Storage(String),
}

impl From<EventStoreError> for PipelineError {
    fn from(err: EventStoreError) -> Self {
        match err {
            EventStoreError::ConcurrencyError { expected, actual } => {
                PipelineError::Concurrency { expected, actual }
            }
            EventStoreError::StorageError(msg) => PipelineError::Storage(msg),
            EventStoreError::SerializationError(err) => PipelineError::Storage(err.to_string()),
            EventStoreError::NotFound => PipelineError::Storage("stream not found".to_string()),
            EventStoreError::Unknown(msg) => PipelineError::Storage(msg),
        }
    }
}

pub struct EventPipeline {
    storage: Arc<dyn EventStore + Send + Sync>,
    workers: Vec<mpsc::Sender<PipelineCommand>>,
    topology: ClusterTopology,
    cluster_client: ClusterClient,
    self_addr: String,
}

impl EventPipeline {
    /// Creates a new pipeline instance, initializing worker pools and cluster topology.
    pub fn new(
        storage: Arc<dyn EventStore + Send + Sync>,
        cluster_nodes: Vec<String>,
        self_node_id: u64,
        auth_token: Option<String>,
    ) -> Self {
        let mut workers = Vec::with_capacity(NUM_WORKERS);

        for id in 0..NUM_WORKERS {
            let (tx, rx) = mpsc::channel::<PipelineCommand>(1024);
            let store = storage.clone();
            let worker = Worker::new(id, store);

            tokio::spawn(async move {
                worker.run(rx).await;
            });
            workers.push(tx);
        }

        // Initialize Topology with Epoch 0 (MVP Static)
        let topology = ClusterTopology::new(cluster_nodes.clone(), 0);

        // Determine self address based on ID, safe fallback if config is weird
        let sorted_nodes = topology.get_all_nodes();
        let self_addr = if (self_node_id as usize) < sorted_nodes.len() {
            sorted_nodes[self_node_id as usize].clone()
        } else {
            // Fallback for single node dev mode if config mismatch, assuming first one
            sorted_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| "127.0.0.1:50051".to_string())
        };

        let cluster_client = ClusterClient::new(auth_token);

        Self {
            storage,
            workers,
            topology,
            cluster_client,
            self_addr,
        }
    }

    /// Appends events to a stream.
    ///
    /// This method acts as the Gateway/Router. It determines if the current node
    /// owns the stream. If so, it processes locally. If not, it forwards the request
    /// to the correct owner via gRPC.
    #[tracing::instrument(skip(self, events), fields(stream_id = %stream_id, event_count = events.len()))]
    pub async fn append_event(
        &self,
        stream_id: &str,
        events: Vec<Event>,
        expected_version: i64,
    ) -> Result<(), PipelineError> {
        if expected_version < -1 {
            return Err(PipelineError::InvalidExpectedVersion(expected_version));
        }

        let owner = self.topology.get_owner(stream_id);

        if owner.node_addr == self.self_addr {
            self.append_event_as_owner(stream_id, events, expected_version)
                .await
        } else {
            match self
                .cluster_client
                .forward_append(&owner.node_addr, stream_id, events, expected_version)
                .await
            {
                Ok(true) => Ok(()),
                Ok(false) => Err(PipelineError::Forwarding {
                    target: owner.node_addr,
                    reason: "peer rejected append".to_string(),
                }),
                Err(reason) => Err(PipelineError::Forwarding {
                    target: owner.node_addr,
                    reason,
                }),
            }
        }
    }

    /// Strict Entry point: Only processes if WE are the owner.
    /// Used for forwarded requests or strict validation.
    pub async fn append_event_as_owner(
        &self,
        stream_id: &str,
        events: Vec<Event>,
        expected_version: i64,
    ) -> Result<(), PipelineError> {
        if expected_version < -1 {
            return Err(PipelineError::InvalidExpectedVersion(expected_version));
        }

        // 1. Validate Ownership Again (Safety)
        let owner = self.topology.get_owner(stream_id);
        if owner.node_addr != self.self_addr {
            return Err(PipelineError::NotOwner {
                stream_id: stream_id.to_string(),
                owner: owner.node_addr,
                current_node: self.self_addr.clone(),
                epoch: owner.epoch,
            });
        }

        // Schema Validation (Soft Fail)
        for event in &events {
            let type_str = format!("{:?}", event.event_type);
            // Optimization: Only check if looks like custom event or check existence
            if let Ok(Some(schema)) = self.storage.get_schema(&type_str).await {
                if let Err(errs) = crate::domain::schema::validation::validate_event_payload(
                    &event.payload.0,
                    &schema,
                ) {
                    tracing::warn!(stream_id = %stream_id, event_type = %type_str, errors = ?errs, "Schema validation failed (Soft Fail)");
                    // To enable Hard Fail: return Err(format!("Schema Validation Error: {:?}", errs));
                }
            }
        }

        // 2. Local Processing via Sharded Workers
        let mut hasher = DefaultHasher::new();
        stream_id.hash(&mut hasher);
        let hash = hasher.finish();
        let worker_idx = (hash as usize) % self.workers.len();

        let (resp_tx, resp_rx) = oneshot::channel();

        let cmd = PipelineCommand::Append {
            stream_id: stream_id.to_string(),
            events,
            expected_version,
            resp_tx,
        };

        self.workers[worker_idx]
            .send(cmd)
            .await
            .map_err(|e| PipelineError::Storage(e.to_string()))?;

        resp_rx
            .await
            .map_err(|e| PipelineError::Storage(e.to_string()))?
    }

    pub async fn fetch_stream(&self, stream_id: &str) -> Result<Vec<Event>, String> {
        self.storage
            .fetch_stream(stream_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn upsert_schema(
        &self,
        schema: crate::domain::schema::model::Schema,
    ) -> Result<(), String> {
        self.storage
            .upsert_schema(schema)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_schema(
        &self,
        name: &str,
    ) -> Result<Option<crate::domain::schema::model::Schema>, String> {
        self.storage
            .get_schema(name)
            .await
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{EventPipeline, PipelineError};
    use crate::domain::events::event::Event;
    use crate::domain::events::event_kind::{EventKind, EventPayload};
    use crate::domain::schema::model::Schema;
    use crate::storage::event_store::{EventStore, EventStoreError};
    use std::sync::Arc;
    use tonic::async_trait;

    struct ConcurrencyStore;

    #[async_trait]
    impl EventStore for ConcurrencyStore {
        async fn append_event(
            &self,
            _stream: &str,
            _event: Event,
            expected_version: u64,
        ) -> Result<(), EventStoreError> {
            Err(EventStoreError::ConcurrencyError {
                expected: expected_version,
                actual: expected_version + 1,
            })
        }

        async fn fetch_stream(&self, _stream: &str) -> Result<Vec<Event>, EventStoreError> {
            Ok(Vec::new())
        }

        async fn upsert_schema(&self, _schema: Schema) -> Result<(), EventStoreError> {
            Ok(())
        }

        async fn get_schema(&self, _name: &str) -> Result<Option<Schema>, EventStoreError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn rejects_invalid_expected_version() {
        let pipeline = EventPipeline::new(
            Arc::new(ConcurrencyStore),
            vec!["127.0.0.1:50051".to_string()],
            0,
            None,
        );

        let event = Event::new("stream-1", EventKind::Internal, EventPayload(vec![1]));
        let result = pipeline.append_event("stream-1", vec![event], -2).await;

        assert!(matches!(
            result,
            Err(PipelineError::InvalidExpectedVersion(-2))
        ));
    }

    #[tokio::test]
    async fn propagates_concurrency_conflicts() {
        let pipeline = EventPipeline::new(
            Arc::new(ConcurrencyStore),
            vec!["127.0.0.1:50051".to_string()],
            0,
            None,
        );

        let event = Event::new("stream-1", EventKind::Internal, EventPayload(vec![1]));
        let result = pipeline
            .append_event_as_owner("stream-1", vec![event], 0)
            .await;

        assert!(matches!(
            result,
            Err(PipelineError::Concurrency {
                expected: 0,
                actual: 1
            })
        ));
    }
}
