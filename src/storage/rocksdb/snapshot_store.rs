use crate::storage::snapshot::{Snapshot, SnapshotError, SnapshotStore};
use rocksdb::DB;
use std::sync::Arc;
use tonic::async_trait;

/// RocksDB-backed snapshot store.
pub struct RocksSnapshotStore {
    db: Arc<DB>,
}

impl RocksSnapshotStore {
    /// Creates a snapshot store using an already opened RocksDB handle.
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SnapshotStore for RocksSnapshotStore {
    async fn save_snapshot(&self, snapshot: Snapshot) -> Result<(), SnapshotError> {
        let key = format!("snapshot:{}", snapshot.stream_id);

        let mut buf = Vec::new();
        // Binary format: [version:8][timestamp:8][payload...]
        buf.extend_from_slice(&snapshot.version.to_be_bytes());
        buf.extend_from_slice(&snapshot.timestamp.to_be_bytes());
        buf.extend_from_slice(&snapshot.payload);

        self.db
            .put(key, buf)
            .map_err(|e| SnapshotError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn get_snapshot(&self, stream_id: &str) -> Result<Option<Snapshot>, SnapshotError> {
        let key = format!("snapshot:{}", stream_id);

        match self
            .db
            .get(key)
            .map_err(|e| SnapshotError::StorageError(e.to_string()))?
        {
            Some(bytes) => {
                if bytes.len() < 16 {
                    return Ok(None);
                }

                let (ver_bytes, rest) = bytes.split_at(8);
                let (ts_bytes, payload) = rest.split_at(8);

                let version = match <[u8; 8]>::try_from(ver_bytes) {
                    Ok(v) => u64::from_be_bytes(v),
                    Err(_) => return Ok(None),
                };
                let timestamp = match <[u8; 8]>::try_from(ts_bytes) {
                    Ok(v) => u64::from_be_bytes(v),
                    Err(_) => return Ok(None),
                };

                Ok(Some(Snapshot {
                    stream_id: stream_id.to_string(),
                    version,
                    payload: payload.to_vec(),
                    timestamp,
                }))
            }
            None => Ok(None),
        }
    }
}
