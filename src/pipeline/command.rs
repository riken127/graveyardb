use crate::domain::events::event::Event;
use crate::pipeline::PipelineError;
use tokio::sync::oneshot;

pub enum PipelineCommand {
    Append {
        stream_id: String,
        events: Vec<Event>,
        expected_version: i64,
        resp_tx: oneshot::Sender<Result<(), PipelineError>>,
    },
}
