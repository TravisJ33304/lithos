//! Client and server message definitions for the Lithos protocol.

use serde::{Deserialize, Serialize};

use crate::types::{EntityId, EntitySnapshot, PlayerId, Vec2, ZoneId};

// ---------------------------------------------------------------------------
// Client → Server
// ---------------------------------------------------------------------------

/// Messages sent from the game client to the server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientMessage {
    /// Request to join the game with a given auth token.
    Join {
        /// Supabase JWT token.
        token: String,
    },

    /// Movement input vector (normalised direction).
    Move {
        /// The direction the player intends to move.
        direction: Vec2,
        /// Client-side sequence number for reconciliation.
        seq: u32,
    },

    /// Request to transfer to a different zone.
    ZoneTransfer {
        /// Target zone.
        target: ZoneId,
    },

    /// Periodic heartbeat / keep-alive.
    Ping {
        /// Client timestamp (ms since epoch).
        timestamp: u64,
    },
}

// ---------------------------------------------------------------------------
// Server → Client
// ---------------------------------------------------------------------------

/// Messages sent from the server to game clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerMessage {
    /// Acknowledgement of a successful join. Provides the client with its IDs.
    JoinAck {
        player_id: PlayerId,
        entity_id: EntityId,
        zone: ZoneId,
    },

    /// A full or delta state snapshot of all visible entities.
    StateSnapshot {
        /// Server tick number.
        tick: u64,
        /// Last client input sequence the server has processed.
        last_processed_seq: u32,
        /// Visible entity states.
        entities: Vec<EntitySnapshot>,
    },

    /// Notification that the player has been transferred to a new zone.
    ZoneChanged {
        zone: ZoneId,
    },

    /// Response to a client ping.
    Pong {
        /// Echoed client timestamp.
        client_timestamp: u64,
        /// Server timestamp (ms since epoch).
        server_timestamp: u64,
    },

    /// The server is kicking the client.
    Disconnect {
        reason: String,
    },
}
