use crate::api;
use crate::domain::events::event::Event;
use crate::domain::events::event_kind::{EventKind, EventPayload};
use std::collections::HashMap;

#[test]
fn test_event_creation_v7() {
    let payload = EventPayload(vec![1, 2, 3]);
    let event = Event::new("stream-1", EventKind::Internal, payload);

    assert_eq!(event.stream_id, "stream-1");
    // Check version 7
    assert_eq!(event.id.0.get_version(), Some(uuid::Version::SortRand));
}

#[test]
fn test_serialization() {
    let payload = EventPayload(vec![1, 2, 3]);
    let event = Event::new("stream-1", EventKind::Internal, payload);

    let serialized = serde_json::to_string(&event).expect("Failed to serialize");
    let deserialized: Event = serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(event.id.0, deserialized.id.0);
    assert_eq!(event.stream_id, deserialized.stream_id);
}

#[test]
fn preserves_custom_event_type_through_proto_conversion() {
    let proto_event = api::Event {
        id: uuid::Uuid::now_v7().to_string(),
        event_type: "UserCreated".to_string(),
        payload: br#"{"name":"Ada"}"#.to_vec(),
        timestamp: 123,
        metadata: HashMap::new(),
    };

    let domain_event: Event = proto_event.try_into().expect("conversion should succeed");
    assert_eq!(
        domain_event.event_type,
        EventKind::Custom("UserCreated".to_string())
    );

    let proto_roundtrip: api::Event = domain_event.into();
    assert_eq!(proto_roundtrip.event_type, "UserCreated");
}
