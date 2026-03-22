use crate::domain::events::event::Event;
use crate::pipeline::command::PipelineCommand;
use crate::pipeline::PipelineError;
use crate::storage::event_store::EventStore;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct Worker {
    _id: usize,
    store: Arc<dyn EventStore + Send + Sync>,
}

impl Worker {
    /// Constructs a worker bound to a shared event store.
    pub fn new(_id: usize, store: Arc<dyn EventStore + Send + Sync>) -> Self {
        Self { _id, store }
    }

    /// Processes serialized commands assigned to this worker.
    pub async fn run(self, mut rx: mpsc::Receiver<PipelineCommand>) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                PipelineCommand::Append {
                    stream_id,
                    mut events,
                    expected_version,
                    resp_tx,
                } => {
                    let res = self
                        .handle_append(&stream_id, &mut events, expected_version)
                        .await;
                    let _ = resp_tx.send(res);
                }
            }
        }
    }

    async fn handle_append(
        &self,
        stream_id: &str,
        events: &mut Vec<Event>,
        expected_version: i64,
    ) -> Result<(), PipelineError> {
        if expected_version < -1 {
            return Err(PipelineError::InvalidExpectedVersion(expected_version));
        }
        if events.len() != 1 {
            return Err(PipelineError::BatchAppendUnsupported {
                stream_id: stream_id.to_string(),
                event_count: events.len(),
            });
        }

        // Resolve the starting expected version.
        let current_version_u64 = if expected_version == -1 {
            let current_events = self
                .store
                .fetch_stream(stream_id)
                .await
                .map_err(PipelineError::from)?;
            current_events
                .last()
                .map(|e| e.sequence_number)
                .unwrap_or(0)
        } else {
            expected_version as u64
        };

        let mut event = events
            .pop()
            .expect("single-event guard should ensure one event is present");
        event.stream_id = stream_id.to_string();

        self.store
            .append_event(stream_id, event, current_version_u64)
            .await
            .map_err(PipelineError::from)?;

        Ok(())
    }
}
