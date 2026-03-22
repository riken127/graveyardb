#[cfg(test)]
mod tests {
    use crate::domain::events::event::Event;
    use crate::domain::events::event_kind::{EventKind, EventPayload};

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
}
