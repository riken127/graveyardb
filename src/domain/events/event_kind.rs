use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        Self(since_the_epoch.as_millis() as u64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPayload(pub Vec<u8>);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventKind {
    Internal,
    Schematic,
    Transactional,
    External,
    Custom(String),
}

impl EventKind {
    pub fn from_type_name(value: &str) -> Self {
        match value {
            "Internal" => Self::Internal,
            "Schematic" => Self::Schematic,
            "Transactional" => Self::Transactional,
            "External" => Self::External,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal => write!(f, "Internal"),
            Self::Schematic => write!(f, "Schematic"),
            Self::Transactional => write!(f, "Transactional"),
            Self::External => write!(f, "External"),
            Self::Custom(value) => write!(f, "{}", value),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaType(pub String);
