//! MessagePack encoding and decoding helpers.
//!
//! All messages are serialized as MessagePack binary frames for efficient
//! transmission over WebSocket connections.
//!
//! Encoding uses MessagePack’s compact struct layout (`rmp_serde::to_vec`).
//! Decoding accepts both compact and map-based layouts (`rmp_serde::from_slice`).

use serde::{Deserialize, Serialize};

/// Errors that can occur during encoding or decoding.
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("failed to encode message: {0}")]
    Encode(#[from] rmp_serde::encode::Error),

    #[error("failed to decode message: {0}")]
    Decode(#[from] rmp_serde::decode::Error),
}

/// Encode a value to a MessagePack byte vector.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError> {
    rmp_serde::to_vec(value).map_err(CodecError::Encode)
}

/// Decode a value from a MessagePack byte slice.
pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, CodecError> {
    rmp_serde::from_slice(bytes).map_err(CodecError::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{ClientMessage, ServerMessage};
    use crate::types::*;

    #[test]
    fn roundtrip_client_join() {
        let msg = ClientMessage::Join {
            token: "test-jwt-token".to_string(),
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ClientMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_client_move() {
        let msg = ClientMessage::Move {
            direction: Vec2::new(1.0, 0.0),
            seq: 42,
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ClientMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_client_zone_transfer() {
        let msg = ClientMessage::ZoneTransfer {
            target: ZoneId::AsteroidBase(7),
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ClientMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_client_ping() {
        let msg = ClientMessage::Ping {
            timestamp: 1_700_000_000_000,
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ClientMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_server_join_ack() {
        let msg = ServerMessage::JoinAck {
            player_id: PlayerId::new(),
            entity_id: EntityId(1),
            zone: ZoneId::Overworld,
            world_seed: 12345,
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_server_state_snapshot() {
        let msg = ServerMessage::StateSnapshot {
            tick: 100,
            last_processed_seq: 40,
            entities: vec![
                EntitySnapshot {
                    id: EntityId(1),
                    position: Vec2::new(10.0, 20.0),
                    velocity: Vec2::ZERO,
                    zone: ZoneId::Overworld,
                    entity_type: crate::types::SnapshotEntityType::Player,
                },
                EntitySnapshot {
                    id: EntityId(2),
                    position: Vec2::new(-5.5, 3.2),
                    velocity: Vec2::new(1.0, -1.0),
                    zone: ZoneId::Overworld,
                    entity_type: crate::types::SnapshotEntityType::Hostile,
                },
            ],
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_server_pong() {
        let msg = ServerMessage::Pong {
            client_timestamp: 1_700_000_000_000,
            server_timestamp: 1_700_000_000_005,
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_server_disconnect() {
        let msg = ServerMessage::Disconnect {
            reason: "server shutting down".to_string(),
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_client_fire() {
        let msg = ClientMessage::Fire {
            direction: Vec2::new(0.5, 0.5),
            client_latency_ms: 45,
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ClientMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_client_respawn() {
        let msg = ClientMessage::Respawn;
        let bytes = encode(&msg).unwrap();
        let decoded: ClientMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_server_health_changed() {
        let msg = ServerMessage::HealthChanged {
            entity_id: EntityId(4),
            health: 50.0,
            max_health: 100.0,
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_server_player_died() {
        let msg = ServerMessage::PlayerDied {
            entity_id: EntityId(5),
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_server_inventory_updated() {
        let msg = ServerMessage::InventoryUpdated {
            entity_id: EntityId(6),
            items_json: "[\"medkit\", \"scrap\"]".to_string(),
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_server_spawn_projectile() {
        let msg = ServerMessage::SpawnProjectile {
            entity_id: EntityId(7),
            position: Vec2::new(10.0, 10.0),
            velocity: Vec2::new(2.0, 0.0),
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_client_chat() {
        let msg = ClientMessage::Chat {
            channel: ChatChannel::Global,
            text: "hello station".to_string(),
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ClientMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn roundtrip_server_chat_message() {
        let msg = ServerMessage::ChatMessage {
            from_entity_id: EntityId(8),
            channel: ChatChannel::Faction,
            text: "brace for breach".to_string(),
            sent_at_unix_ms: 1_700_000_010_000,
        };
        let bytes = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }
}
