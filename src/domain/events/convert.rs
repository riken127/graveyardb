use crate::api as proto;
use crate::domain::events::event::Event;
use crate::domain::events::event_kind::{EventId, EventKind, EventPayload, Timestamp};

impl TryFrom<proto::Event> for Event {
    type Error = String;

    fn try_from(proto_event: proto::Event) -> Result<Self, Self::Error> {
        // Preserve unknown/custom event type strings so schema lookups remain accurate.
        let event_type = EventKind::from_type_name(&proto_event.event_type);

        use std::str::FromStr;
        Ok(Event {
            id: EventId(uuid::Uuid::from_str(&proto_event.id).map_err(|e| e.to_string())?),
            stream_id: String::new(),
            sequence_number: 0,
            event_type,
            payload: EventPayload(proto_event.payload),
            timestamp: Timestamp(proto_event.timestamp),
            metadata: proto_event.metadata,
        })
    }
}

impl From<Event> for proto::Event {
    fn from(domain_event: Event) -> Self {
        proto::Event {
            id: domain_event.id.0.to_string(),
            event_type: domain_event.event_type.to_string(),
            payload: domain_event.payload.0,
            timestamp: domain_event.timestamp.0,
            metadata: domain_event.metadata,
        }
    }
}
