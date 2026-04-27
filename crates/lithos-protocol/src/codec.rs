//! MessagePack encoding and decoding helpers.
//!
//! All messages are serialized as MessagePack binary frames for efficient
//! transmission over WebSocket connections.

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
    rmp_serde::to_vec_named(value).map_err(CodecError::Encode)
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
                },
                EntitySnapshot {
                    id: EntityId(2),
                    position: Vec2::new(-5.5, 3.2),
                    velocity: Vec2::new(1.0, -1.0),
                    zone: ZoneId::Overworld,
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
}
