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

        let snapshot = crate::storage::snapshot::Snapshot {
            stream_id: proto_snap.stream_id,
            version: proto_snap.version,
            payload: proto_snap.payload,
            timestamp: proto_snap.timestamp,
        };

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

#[cfg(test)]
mod tests {
    use super::{
        status_from_pipeline_error, status_from_schema_upsert_error, validate_expected_version,
    };
    use crate::pipeline::{PipelineError, SchemaUpsertError};
    use tonic::Code;

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
}
