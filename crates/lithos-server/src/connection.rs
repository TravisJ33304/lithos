//! Connection management — tracks connected clients and their metadata.

use bytes::Bytes;
use std::collections::HashMap;
use tokio::sync::mpsc;

use lithos_protocol::{EntityId, PlayerId};

/// A connected client's handle.
#[derive(Debug)]
pub struct ClientConnection {
    #[allow(dead_code)]
    pub player_id: PlayerId,
    pub entity_id: EntityId,
    pub faction_id: Option<u64>,
    /// The player's username (used for DB persistence).
    pub username: String,
    /// Channel to send serialized messages back to this client's WebSocket task.
    pub outbound_tx: mpsc::UnboundedSender<Bytes>,
}

/// Manages all connected clients.
#[derive(Debug, Default)]
pub struct ConnectionManager {
    clients: HashMap<EntityId, ClientConnection>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new client. Returns the assigned EntityId.
    pub fn add(
        &mut self,
        player_id: PlayerId,
        entity_id: EntityId,
        faction_id: Option<u64>,
        username: String,
        outbound_tx: mpsc::UnboundedSender<Bytes>,
    ) {
        self.clients.insert(
            entity_id,
            ClientConnection {
                player_id,
                entity_id,
                faction_id,
                username,
                outbound_tx,
            },
        );
        tracing::info!(
            player_id = %player_id.0,
            entity_id = entity_id.0,
            total = self.clients.len(),
            "client connected"
        );
    }

    /// Remove a client by entity ID, returning the connection if it existed.
    pub fn remove(&mut self, entity_id: EntityId) -> Option<ClientConnection> {
        let removed = self.clients.remove(&entity_id);
        if removed.is_some() {
            tracing::info!(
                entity_id = entity_id.0,
                total = self.clients.len(),
                "client disconnected"
            );
        }
        removed
    }

    /// Iterate over all connected clients.
    pub fn iter(&self) -> impl Iterator<Item = &ClientConnection> {
        self.clients.values()
    }

    /// Get the number of connected clients.
    pub fn count(&self) -> usize {
        self.clients.len()
    }

    /// Get a single connection by entity ID.
    pub fn get(&self, entity_id: EntityId) -> Option<&ClientConnection> {
        self.clients.get(&entity_id)
    }
}
