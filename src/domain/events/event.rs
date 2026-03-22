use crate::domain::events::event_kind::{EventId, EventKind, EventPayload, Timestamp};

use serde::{Deserialize, Serialize};

/// Mandatory state change semantics attached to each event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transition {
    /// Transition identifier (for example, `user.activated`).
    pub name: String,
    /// State before the transition is applied.
    pub from_state: String,
    /// State after the transition is applied.
    pub to_state: String,
}

impl Transition {
    /// Creates a new transition description.
    pub fn new(
        name: impl Into<String>,
        from_state: impl Into<String>,
        to_state: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            from_state: from_state.into(),
            to_state: to_state.into(),
        }
    }

    /// Validates transition semantics before persistence.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("transition name must not be empty".to_string());
        }
        if self.from_state.trim().is_empty() {
            return Err("transition from_state must not be empty".to_string());
        }
        if self.to_state.trim().is_empty() {
            return Err("transition to_state must not be empty".to_string());
        }
        if self.from_state == self.to_state {
            return Err("transition from_state and to_state must be different".to_string());
        }

        Ok(())
    }
}

impl Default for Transition {
    fn default() -> Self {
        Self::new("legacy.unknown", "unknown", "unknown")
    }
}

/// Represents an Event in the Graveyar_DB system.
///
/// An event is an immutable record of something that happened in the domain.
/// It contains a unique ID, a stream ID, a sequence number within that stream,
/// the event type/kind, the data payload, a timestamp, transition semantics, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique identifier for the event.
    pub id: EventId,

    /// The ID of the stream this event belongs to.
    pub stream_id: String,

    /// The monotonic sequence number of this event within its stream.
    /// This is assigned by the storage engine upon persistence.
    pub sequence_number: u64,

    /// The type of the event (e.g., "UserCreated").
    pub event_type: EventKind,

    /// The binary payload of the event.
    pub payload: EventPayload,

    /// The wall-clock time when the event was created/ingested.
    pub timestamp: Timestamp,

    /// Mandatory state transition encoded by this event.
    #[serde(default)]
    pub transition: Transition,

    /// Additional context key-value pairs (e.g., Tracing info, Saga state).
    /// This allows evolution of process logic without changing the payload schema.
    pub metadata: std::collections::HashMap<String, String>,
}

impl Event {
    /// Creates a new `Event` instance with a generated ID and timestamp.
    /// The `sequence_number` is initialized to 0 and should be set during persistence.
    pub fn new(
        stream_id: impl Into<String>,
        event_type: EventKind,
        payload: EventPayload,
        transition: Transition,
    ) -> Self {
        Self {
            id: EventId::new(),
            stream_id: stream_id.into(),
            sequence_number: 0, // Assigned by storage
            event_type,
            payload,
            timestamp: Timestamp::now(),
            transition,
            metadata: std::collections::HashMap::new(),
        }
    }
}
