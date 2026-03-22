use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::api::{
    event_store_server::EventStore, AppendEventRequest, AppendEventResponse, Event as ProtoEvent,
    GetEventsRequest, GetSchemaRequest, GetSchemaResponse, UpsertSchemaRequest,
    UpsertSchemaResponse,
};
use crate::domain::events::event::Event as DomainEvent;
use crate::pipeline::{EventPipeline, PipelineError, SchemaUpsertError};
use crate::storage::snapshot::Snapshot;

pub mod auth;

pub struct GrpcService {
    pipeline: Arc<EventPipeline>,
    snapshot_store: Arc<dyn crate::storage::snapshot::SnapshotStore>,
}

impl GrpcService {
    pub fn new(
        pipeline: Arc<EventPipeline>,
        snapshot_store: Arc<dyn crate::storage::snapshot::SnapshotStore>,
    ) -> Self {
        Self {
            pipeline,
            snapshot_store,
        }
    }
}

#[tonic::async_trait]
impl EventStore for GrpcService {
    // Defines the stream generic for GetEvents for clarity
    type GetEventsStream = ReceiverStream<Result<ProtoEvent, Status>>;

    async fn append_event(
        &self,
        request: Request<AppendEventRequest>,
    ) -> Result<Response<AppendEventResponse>, Status> {
        let req = request.into_inner();
        let stream_id = req.stream_id;
        validate_expected_version(req.expected_version)?;
        let expected_version = req.expected_version;

        // Convert proto events to domain events
        let mut domain_events = Vec::new();
        for proto_event in req.events {
            // using TryFrom
            let mut event: DomainEvent = proto_event
                .try_into()
                .map_err(|e: String| Status::invalid_argument(e))?;
            // Ensure stream_id is set
            event.stream_id = stream_id.clone();
            domain_events.push(event);
        }

        let is_forwarded = req.is_forwarded;

        let result = if is_forwarded {
            self.pipeline
                .append_event_as_owner(&stream_id, domain_events, expected_version)
                .await
        } else {
            self.pipeline
                .append_event(&stream_id, domain_events, expected_version)
                .await
        };

        result.map_err(status_from_pipeline_error)?;

        Ok(Response::new(AppendEventResponse { success: true }))
    }

    async fn get_events(
        &self,
        request: Request<GetEventsRequest>,
    ) -> Result<Response<Self::GetEventsStream>, Status> {
        let req = request.into_inner();
        let stream_id = req.stream_id;

        let events = self
            .pipeline
            .fetch_stream(&stream_id)
            .await
            .map_err(Status::internal)?;

        let (tx, rx) = mpsc::channel(128);

        // Spawn sender
        tokio::spawn(async move {
            for event in events {
                let proto_event: ProtoEvent = event.into();
                if (tx.send(Ok(proto_event)).await).is_err() {
                    break; // Receiver closed
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn upsert_schema(
        &self,
        request: Request<UpsertSchemaRequest>,
    ) -> Result<Response<UpsertSchemaResponse>, Status> {
        let req = request.into_inner();
        let proto_schema = req
            .schema
            .ok_or_else(|| Status::invalid_argument("Schema is required"))?;

        let schema: crate::domain::schema::model::Schema = proto_schema.into();

        self.pipeline
            .upsert_schema(schema)
            .await
            .map_err(status_from_schema_upsert_error)?;

        Ok(Response::new(UpsertSchemaResponse {
            success: true,
            message: "Schema upserted".to_string(),
        }))
    }

    async fn get_schema(
        &self,
        request: Request<GetSchemaRequest>,
    ) -> Result<Response<GetSchemaResponse>, Status> {
        let req = request.into_inner();
        let name = req.name;

        let schema_opt: Option<crate::domain::schema::model::Schema> = self
            .pipeline
            .get_schema(&name)
            .await
            .map_err(Status::internal)?;

        match schema_opt {
            Some(schema) => {
                let proto_schema: crate::api::Schema = schema.into();
                Ok(Response::new(GetSchemaResponse {
                    schema: Some(proto_schema),
                    found: true,
                }))
            }
            None => Ok(Response::new(GetSchemaResponse {
                schema: None,
                found: false,
            })),
        }
    }

    async fn save_snapshot(
        &self,
        request: Request<crate::api::SaveSnapshotRequest>,
    ) -> Result<Response<crate::api::SaveSnapshotResponse>, Status> {
        let req = request.into_inner();
        let proto_snap = req
            .snapshot
            .ok_or_else(|| Status::invalid_argument("Missing snapshot"))?;

        let snapshot = Snapshot {
            stream_id: proto_snap.stream_id,
            version: proto_snap.version,
            payload: proto_snap.payload,
            timestamp: proto_snap.timestamp,
        };

        let current_stream_version = self
            .pipeline
            .fetch_stream(&snapshot.stream_id)
            .await
            .map_err(Status::internal)?
            .last()
            .map(|event| event.sequence_number)
            .unwrap_or(0);

        let existing_snapshot = self
            .snapshot_store
            .get_snapshot(&snapshot.stream_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        validate_snapshot_write(
            &snapshot,
            current_stream_version,
            existing_snapshot.as_ref(),
        )?;

        self.snapshot_store
            .save_snapshot(snapshot)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(crate::api::SaveSnapshotResponse {
            success: true,
        }))
    }

    async fn get_snapshot(
        &self,
        request: Request<crate::api::GetSnapshotRequest>,
    ) -> Result<Response<crate::api::GetSnapshotResponse>, Status> {
        let req = request.into_inner();

        let snap_opt = self
            .snapshot_store
            .get_snapshot(&req.stream_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        match snap_opt {
            Some(s) => {
                let proto_snap = crate::api::Snapshot {
                    stream_id: s.stream_id,
                    version: s.version,
                    payload: s.payload,
                    timestamp: s.timestamp,
                };
                Ok(Response::new(crate::api::GetSnapshotResponse {
                    snapshot: Some(proto_snap),
                    found: true,
                }))
            }
            None => Ok(Response::new(crate::api::GetSnapshotResponse {
                snapshot: None,
                found: false,
            })),
        }
    }
}

fn validate_expected_version(expected_version: i64) -> Result<(), Status> {
    if expected_version < -1 {
        return Err(Status::invalid_argument(
            "expected_version must be -1 or a non-negative version",
        ));
    }

    Ok(())
}

fn status_from_pipeline_error(error: PipelineError) -> Status {
    match error {
        PipelineError::InvalidExpectedVersion(value) => Status::invalid_argument(format!(
            "expected_version must be -1 or a non-negative version, got {}",
            value
        )),
        PipelineError::BatchAppendUnsupported {
            stream_id,
            event_count,
        } => Status::invalid_argument(format!(
            "batch append of {} events for stream {} is not supported yet",
            event_count, stream_id
        )),
        PipelineError::NotOwner {
            stream_id,
            owner,
            current_node,
            epoch,
        } => Status::failed_precondition(format!(
            "stream {} is owned by {} not {} (epoch {})",
            stream_id, owner, current_node, epoch
        )),
        PipelineError::Concurrency { expected, actual } => Status::aborted(format!(
            "expected version {} does not match current version {}",
            expected, actual
        )),
        PipelineError::SchemaLookup {
            stream_id,
            event_type,
            reason,
        } => Status::internal(format!(
            "schema lookup failed for stream {}, event {}: {}",
            stream_id, event_type, reason
        )),
        PipelineError::Forwarding { target, reason } => Status::unavailable(format!(
            "failed to forward append to {}: {}",
            target, reason
        )),
        PipelineError::SchemaValidation {
            stream_id,
            event_type,
            details,
        } => Status::failed_precondition(format!(
            "schema validation failed for stream {}, event {}: {}",
            stream_id, event_type, details
        )),
        PipelineError::TransitionValidation {
            stream_id,
            event_type,
            event_index,
            details,
        } => Status::invalid_argument(format!(
            "transition validation failed for stream {}, event {} at index {}: {}",
            stream_id, event_type, event_index, details
        )),
        PipelineError::Storage(msg) => Status::internal(msg),
    }
}

fn status_from_schema_upsert_error(error: SchemaUpsertError) -> Status {
    match error {
        SchemaUpsertError::InvalidContract { details } => Status::invalid_argument(details),
        SchemaUpsertError::Storage(msg) => Status::internal(msg),
    }
}

fn validate_snapshot_write(
    snapshot: &Snapshot,
    current_stream_version: u64,
    existing_snapshot: Option<&Snapshot>,
) -> Result<(), Status> {
    if snapshot.version > current_stream_version {
        return Err(Status::invalid_argument(format!(
            "snapshot version {} is ahead of current stream version {} for stream {}",
            snapshot.version, current_stream_version, snapshot.stream_id
        )));
    }

    if snapshot.version < current_stream_version {
        return Err(Status::invalid_argument(format!(
            "snapshot version {} is stale for current stream version {} on stream {}",
            snapshot.version, current_stream_version, snapshot.stream_id
        )));
    }

    if let Some(existing) = existing_snapshot {
        if snapshot.version < existing.version {
            return Err(Status::invalid_argument(format!(
                "snapshot version {} is older than existing snapshot version {} for stream {}",
                snapshot.version, existing.version, snapshot.stream_id
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        status_from_pipeline_error, status_from_schema_upsert_error, validate_expected_version,
        validate_snapshot_write, GrpcService,
    };
    use crate::api::event_store_server::EventStore as GrpcEventStore;
    use crate::domain::events::event::{Event, Transition};
    use crate::domain::events::event_kind::{EventKind, EventPayload};
    use crate::pipeline::{PipelineError, SchemaUpsertError};
    use crate::storage::memory::InMemoryEventStore;
    use crate::storage::snapshot::{Snapshot, SnapshotError, SnapshotStore};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tonic::{Code, Request};

    struct MemorySnapshotStore {
        snapshots: Arc<RwLock<HashMap<String, Snapshot>>>,
    }

    impl MemorySnapshotStore {
        fn new() -> Self {
            Self {
                snapshots: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[tonic::async_trait]
    impl SnapshotStore for MemorySnapshotStore {
        async fn save_snapshot(&self, snapshot: Snapshot) -> Result<(), SnapshotError> {
            self.snapshots
                .write()
                .await
                .insert(snapshot.stream_id.clone(), snapshot);
            Ok(())
        }

        async fn get_snapshot(&self, stream_id: &str) -> Result<Option<Snapshot>, SnapshotError> {
            Ok(self.snapshots.read().await.get(stream_id).cloned())
        }
    }

    fn test_service() -> GrpcService {
        let storage = Arc::new(InMemoryEventStore::new());
        let pipeline = Arc::new(crate::pipeline::EventPipeline::new_with_transport(
            storage,
            vec!["127.0.0.1:50051".to_string()],
            0,
            None,
            false,
            std::time::Duration::from_secs(3),
            false,
        ));
        GrpcService::new(pipeline, Arc::new(MemorySnapshotStore::new()))
    }

    #[test]
    fn rejects_expected_version_below_sentinel() {
        let err = validate_expected_version(-2).unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }

    #[test]
    fn maps_concurrency_conflict_to_aborted() {
        let status = status_from_pipeline_error(PipelineError::Concurrency {
            expected: 2,
            actual: 3,
        });

        assert_eq!(status.code(), Code::Aborted);
    }

    #[test]
    fn maps_transition_validation_to_invalid_argument() {
        let status = status_from_pipeline_error(PipelineError::TransitionValidation {
            stream_id: "stream-1".to_string(),
            event_type: "UserCreated".to_string(),
            event_index: 0,
            details: "transition name must not be empty".to_string(),
        });

        assert_eq!(status.code(), Code::InvalidArgument);
    }

    #[test]
    fn maps_schema_contract_errors_to_invalid_argument() {
        let status = status_from_schema_upsert_error(SchemaUpsertError::InvalidContract {
            details: "field age applies numeric constraints to a non-number type".to_string(),
        });
        assert_eq!(status.code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn rejects_future_and_stale_snapshot_versions() {
        let snapshot = Snapshot {
            stream_id: "stream-1".to_string(),
            version: 5,
            payload: Vec::new(),
            timestamp: 0,
        };

        let err = validate_snapshot_write(&snapshot, 4, None).unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);

        let err = validate_snapshot_write(&snapshot, 6, None).unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn save_snapshot_rejects_versions_that_are_ahead_or_behind_the_stream() {
        let service = test_service();
        let event = Event::new(
            "stream-1",
            EventKind::Internal,
            EventPayload(vec![1]),
            Transition::new("created", "none", "active"),
        );

        service
            .pipeline
            .append_event_as_owner("stream-1", vec![event], 0)
            .await
            .expect("append should succeed");

        let future_snapshot = crate::api::Snapshot {
            stream_id: "stream-1".to_string(),
            version: 2,
            payload: vec![1, 2, 3],
            timestamp: 1,
        };
        let response = service
            .save_snapshot(Request::new(crate::api::SaveSnapshotRequest {
                snapshot: Some(future_snapshot),
            }))
            .await;
        assert!(matches!(response, Err(status) if status.code() == Code::InvalidArgument));

        let valid_snapshot = crate::api::Snapshot {
            stream_id: "stream-1".to_string(),
            version: 1,
            payload: vec![4, 5, 6],
            timestamp: 2,
        };
        service
            .snapshot_store
            .save_snapshot(Snapshot {
                stream_id: "stream-1".to_string(),
                version: 2,
                payload: vec![9],
                timestamp: 0,
            })
            .await
            .expect("seed snapshot");
        let response = service
            .save_snapshot(Request::new(crate::api::SaveSnapshotRequest {
                snapshot: Some(valid_snapshot),
            }))
            .await;
        assert!(matches!(response, Err(status) if status.code() == Code::InvalidArgument));
    }
}
