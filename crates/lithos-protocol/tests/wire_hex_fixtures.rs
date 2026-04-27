use lithos_protocol::codec;
use lithos_protocol::messages::ServerMessage;
use lithos_protocol::types::{
    EntityId, EntitySnapshot, PlayerId, SnapshotEntityType, Vec2, ZoneId,
};
use uuid::Uuid;

const PONG_HEX: &str = "81a4506f6e67920102";

const JOIN_ACK_HEX: &str =
    "81a74a6f696e41636b94c410000102030405060708090a0b0c0d0e0f01a94f766572776f726c642a";

const STATE_SNAPSHOT_HEX: &str = "81ad5374617465536e617073686f7493030491950592ca3f800000ca4000000092ca00000000ca00000000a94f766572776f726c64a6506c61796572";

#[test]
fn compact_server_wire_matches_documented_hex() {
    let pong = ServerMessage::Pong {
        client_timestamp: 1,
        server_timestamp: 2,
    };
    assert_eq!(hex::encode(codec::encode(&pong).unwrap()), PONG_HEX);

    let join = ServerMessage::JoinAck {
        player_id: PlayerId(Uuid::from_bytes([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        ])),
        entity_id: EntityId(1),
        zone: ZoneId::Overworld,
        world_seed: 42,
    };
    assert_eq!(hex::encode(codec::encode(&join).unwrap()), JOIN_ACK_HEX);

    let snap = ServerMessage::StateSnapshot {
        tick: 3,
        last_processed_seq: 4,
        entities: vec![EntitySnapshot {
            id: EntityId(5),
            position: Vec2::new(1.0, 2.0),
            velocity: Vec2::ZERO,
            zone: ZoneId::Overworld,
            entity_type: SnapshotEntityType::Player,
        }],
    };
    assert_eq!(
        hex::encode(codec::encode(&snap).unwrap()),
        STATE_SNAPSHOT_HEX
    );
}

#[test]
fn hex_fixture_roundtrips_decode() {
    for (label, hex_s) in [
        ("pong", PONG_HEX),
        ("join_ack", JOIN_ACK_HEX),
        ("state_snapshot", STATE_SNAPSHOT_HEX),
    ] {
        let bytes = hex::decode(hex_s).unwrap();
        let _: ServerMessage = codec::decode(&bytes).unwrap_or_else(|e| {
            panic!("{label}: decode failed: {e}");
        });
    }
}
