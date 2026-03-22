use tonic::async_trait;

/// Point-in-time materialized state for a stream.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Stream identifier.
    pub stream_id: String,
    /// Stream version covered by this snapshot.
    pub version: u64,
    /// Serialized aggregate state payload.
    pub payload: Vec<u8>,
    /// Snapshot creation timestamp in milliseconds since Unix epoch.
    pub timestamp: u64,
}

/// Snapshot storage abstraction errors.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_cbor::Error),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Snapshot persistence interface.
#[async_trait]
pub trait SnapshotStore: Send + Sync {
    /// Stores or replaces the latest snapshot for a stream.
    async fn save_snapshot(&self, snapshot: Snapshot) -> Result<(), SnapshotError>;
    /// Loads the latest snapshot for `stream_id`, if one exists.
    async fn get_snapshot(&self, stream_id: &str) -> Result<Option<Snapshot>, SnapshotError>;
}
