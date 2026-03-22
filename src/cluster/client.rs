use crate::api::event_store_client::EventStoreClient;
use crate::api::{AppendEventRequest, Event as ProtoEvent};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Channel;

/// Reusable gRPC client pool for forwarding requests to peer nodes.
#[derive(Clone)]
pub struct ClusterClient {
    clients: Arc<RwLock<HashMap<String, EventStoreClient<Channel>>>>,
    auth_token: Option<String>,
}

impl ClusterClient {
    /// Creates a forwarding client pool.
    pub fn new(auth_token: Option<String>) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            auth_token,
        }
    }
}

impl Default for ClusterClient {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ClusterClient {
    /// Returns a connected gRPC client for the given node address.
    ///
    /// The method performs a lock-free read first, then upgrades to a write lock
    /// only when a new connection must be established.
    pub async fn get_client(&self, addr: &str) -> Result<EventStoreClient<Channel>, String> {
        {
            let map = self.clients.read().await;
            if let Some(client) = map.get(addr) {
                return Ok(client.clone());
            }
        }

        let mut map = self.clients.write().await;
        if let Some(client) = map.get(addr) {
            return Ok(client.clone());
        }

        let uri = format!("http://{}", addr);
        let channel = Channel::from_shared(uri)
            .map_err(|e| e.to_string())?
            .connect()
            .await
            .map_err(|e| format!("Failed to connect to peer {}: {}", addr, e))?;

        let client = EventStoreClient::new(channel);
        map.insert(addr.to_string(), client.clone());

        Ok(client)
    }

    /// Forwards an append request to the owner node.
    ///
    /// The forwarded request is marked with `is_forwarded=true` so the receiver
    /// executes owner-only append logic and does not forward again.
    pub async fn forward_append(
        &self,
        target_node: &str,
        stream_id: &str,
        events: Vec<crate::domain::events::event::Event>,
        expected_version: i64,
    ) -> Result<bool, String> {
        let mut client = self.get_client(target_node).await?;

        let proto_events: Vec<ProtoEvent> = events.into_iter().map(|e| e.into()).collect();

        let req = AppendEventRequest {
            stream_id: stream_id.to_string(),
            events: proto_events,
            expected_version,
            is_forwarded: true,
        };

        let mut request = tonic::Request::new(req);
        if let Some(token) = &self.auth_token {
            let auth_value = format!("Bearer {}", token);
            if let Ok(meta_val) = tonic::metadata::MetadataValue::from_str(&auth_value) {
                request.metadata_mut().insert("authorization", meta_val);
            }
        }

        let resp = client
            .append_event(request)
            .await
            .map_err(|e| e.to_string())?
            .into_inner();

        Ok(resp.success)
    }
}
