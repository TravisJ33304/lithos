use lithos_protocol::codec;
use lithos_protocol::messages::{ClientMessage, ServerMessage};
use lithos_protocol::types::{
    ChatChannel, DynamicEventKind, DynamicEventSnapshot, EntityId, EntitySnapshot, PlayerId,
    ProgressionSnapshot, RaidStateSnapshot, SkillBranch, SnapshotEntityType, TraderQuote, Vec2,
    ZoneId,
};

fn decode_value(bytes: &[u8]) -> rmpv::Value {
    rmpv::decode::read_value(&mut std::io::Cursor::new(bytes)).unwrap()
}

#[test]
fn compact_join_ack_state_move_shapes() {
    let join_ack = ServerMessage::JoinAck {
        player_id: PlayerId::new(),
        entity_id: EntityId(1),
        zone: ZoneId::AsteroidBase(9),
        world_seed: 42,
    };
    let v = decode_value(&codec::encode(&join_ack).unwrap());
    let rmpv::Value::Map(m) = v else {
        panic!("expected map");
    };
    assert_eq!(m.len(), 1);

    let snap = ServerMessage::StateSnapshot {
        tick: 1,
        last_processed_seq: 2,
        entities: vec![EntitySnapshot {
            id: EntityId(3),
            position: Vec2::new(1.0, 2.0),
            velocity: Vec2::ZERO,
            zone: ZoneId::Overworld,
            entity_type: SnapshotEntityType::Player,
        }],
    };
    let v2 = decode_value(&codec::encode(&snap).unwrap());
    let rmpv::Value::Map(m2) = v2 else {
        panic!("expected map");
    };
    assert_eq!(m2.len(), 1);

    let cm = ClientMessage::Move {
        direction: Vec2::new(0.5, -0.5),
        seq: 7,
    };
    let v3 = decode_value(&codec::encode(&cm).unwrap());
    let rmpv::Value::Map(m3) = v3 else {
        panic!("expected map");
    };
    assert_eq!(m3.len(), 1);
}

#[test]
fn compact_misc_server_message_shapes() {
    let cases: Vec<(&str, ServerMessage)> = vec![
        (
            "ZoneChanged",
            ServerMessage::ZoneChanged {
                zone: ZoneId::Overworld,
            },
        ),
        (
            "HealthChanged",
            ServerMessage::HealthChanged {
                entity_id: EntityId(4),
                health: 10.0,
                max_health: 20.0,
            },
        ),
        (
            "PlayerDied",
            ServerMessage::PlayerDied {
                entity_id: EntityId(5),
            },
        ),
        (
            "InventoryUpdated",
            ServerMessage::InventoryUpdated {
                entity_id: EntityId(6),
                items_json: "[]".into(),
            },
        ),
        (
            "SpawnProjectile",
            ServerMessage::SpawnProjectile {
                entity_id: EntityId(7),
                position: Vec2::new(1.0, 2.0),
                velocity: Vec2::new(3.0, 4.0),
            },
        ),
        (
            "ChatMessage",
            ServerMessage::ChatMessage {
                from_entity_id: EntityId(8),
                channel: ChatChannel::Faction,
                text: "hi".into(),
                sent_at_unix_ms: 99,
            },
        ),
        (
            "CreditsChanged",
            ServerMessage::CreditsChanged {
                faction_id: 11,
                balance: -3,
            },
        ),
        (
            "TraderQuotes",
            ServerMessage::TraderQuotes {
                quotes: vec![TraderQuote {
                    trader_entity_id: EntityId(1),
                    item: "ore".into(),
                    buy_price: 1.0,
                    sell_price: 2.0,
                    demand_scalar: 1.5,
                    available_credits: 100,
                    daily_credit_limit: 5000,
                    daily_credits_used: 0,
                }],
            },
        ),
        (
            "ProgressionUpdated",
            ServerMessage::ProgressionUpdated {
                entity_id: EntityId(9),
                branches: vec![ProgressionSnapshot {
                    branch: SkillBranch::Fabrication,
                    level: 1,
                    xp: 2,
                    xp_to_next: 10,
                }],
            },
        ),
        (
            "DynamicEventStarted",
            ServerMessage::DynamicEventStarted {
                event: DynamicEventSnapshot {
                    event_id: 1,
                    kind: DynamicEventKind::MeteorShower,
                    started_at_unix_ms: 1,
                    expires_at_unix_ms: 2,
                    description: "d".into(),
                },
            },
        ),
        (
            "DynamicEventEnded",
            ServerMessage::DynamicEventEnded { event_id: 3 },
        ),
        (
            "RaidWarning",
            ServerMessage::RaidWarning {
                raid: RaidStateSnapshot {
                    attacker_faction_id: 1,
                    defender_faction_id: 2,
                    warning_remaining_seconds: 5,
                    breach_active: false,
                },
            },
        ),
        (
            "RaidStarted",
            ServerMessage::RaidStarted {
                raid: RaidStateSnapshot {
                    attacker_faction_id: 1,
                    defender_faction_id: 2,
                    warning_remaining_seconds: 0,
                    breach_active: true,
                },
            },
        ),
        (
            "RaidEnded",
            ServerMessage::RaidEnded {
                raid: RaidStateSnapshot {
                    attacker_faction_id: 1,
                    defender_faction_id: 2,
                    warning_remaining_seconds: 0,
                    breach_active: false,
                },
                attacker_won: true,
            },
        ),
        (
            "Pong",
            ServerMessage::Pong {
                client_timestamp: 1,
                server_timestamp: 2,
            },
        ),
        (
            "Disconnect",
            ServerMessage::Disconnect {
                reason: "bye".into(),
            },
        ),
    ];

    for (name, msg) in cases {
        let v = decode_value(&codec::encode(&msg).unwrap());
        let rmpv::Value::Map(m) = v else {
            panic!("{name}: expected map");
        };
        assert_eq!(m.len(), 1, "{name}");
    }
}

#[test]
fn disconnect_payload_is_single_element_array() {
    let b = codec::encode(&ServerMessage::Disconnect {
        reason: "bye".into(),
    })
    .unwrap();
    let v = decode_value(&b);
    let rmpv::Value::Map(m) = v else {
        panic!("expected map");
    };
    let (_, inner) = &m[0];
    let rmpv::Value::Array(a) = inner else {
        panic!("Disconnect inner: {inner:?}");
    };
    assert_eq!(a.len(), 1);
}

#[test]
fn trader_quotes_progression_dynamic_raid_shapes() {
    let tq = ServerMessage::TraderQuotes {
        quotes: vec![TraderQuote {
            trader_entity_id: EntityId(1),
            item: "x".into(),
            buy_price: 1.0,
            sell_price: 2.0,
            demand_scalar: 3.0,
            available_credits: 4,
            daily_credit_limit: 5000,
            daily_credits_used: 10,
        }],
    };
    let v = decode_value(&codec::encode(&tq).unwrap());
    let rmpv::Value::Map(m) = v else {
        panic!("expected map");
    };
    let (_, inner) = &m[0];
    let rmpv::Value::Array(outer) = inner else {
        panic!("TraderQuotes inner: {inner:?}");
    };
    assert_eq!(outer.len(), 1);

    let prog = ServerMessage::ProgressionUpdated {
        entity_id: EntityId(2),
        branches: vec![ProgressionSnapshot {
            branch: SkillBranch::Extraction,
            level: 3,
            xp: 4,
            xp_to_next: 5,
        }],
    };
    let v2 = decode_value(&codec::encode(&prog).unwrap());
    let rmpv::Value::Map(m2) = v2 else {
        panic!("expected map");
    };
    let (_, inner2) = &m2[0];
    let rmpv::Value::Array(arr2) = inner2 else {
        panic!("ProgressionUpdated inner: {inner2:?}");
    };
    assert_eq!(arr2.len(), 2);

    let ev = ServerMessage::DynamicEventStarted {
        event: DynamicEventSnapshot {
            event_id: 10,
            kind: DynamicEventKind::SolarFlare,
            started_at_unix_ms: 11,
            expires_at_unix_ms: 12,
            description: "d".into(),
        },
    };
    let v3 = decode_value(&codec::encode(&ev).unwrap());
    let rmpv::Value::Map(m3) = v3 else {
        panic!("expected map");
    };
    let (_, inner3) = &m3[0];
    let rmpv::Value::Array(arr3) = inner3 else {
        panic!("DynamicEventStarted inner: {inner3:?}");
    };
    assert_eq!(arr3.len(), 1);

    let raid_end = ServerMessage::RaidEnded {
        raid: RaidStateSnapshot {
            attacker_faction_id: 1,
            defender_faction_id: 2,
            warning_remaining_seconds: 3,
            breach_active: true,
        },
        attacker_won: false,
    };
    let v4 = decode_value(&codec::encode(&raid_end).unwrap());
    let rmpv::Value::Map(m4) = v4 else {
        panic!("expected map");
    };
    let (_, inner4) = &m4[0];
    let rmpv::Value::Array(arr4) = inner4 else {
        panic!("RaidEnded inner: {inner4:?}");
    };
    assert_eq!(arr4.len(), 2);
}

#[test]
fn zone_changed_and_player_died_inner_shapes() {
    let z = decode_value(
        &codec::encode(&ServerMessage::ZoneChanged {
            zone: ZoneId::Overworld,
        })
        .unwrap(),
    );
    let rmpv::Value::Map(m) = z else {
        panic!("expected map");
    };
    let (_, inner) = &m[0];
    let rmpv::Value::Array(za) = inner else {
        panic!("ZoneChanged Overworld: {inner:?}");
    };
    assert_eq!(za.len(), 1);
    assert!(matches!(&za[0], rmpv::Value::String(_)));

    let z2 = decode_value(
        &codec::encode(&ServerMessage::ZoneChanged {
            zone: ZoneId::AsteroidBase(3),
        })
        .unwrap(),
    );
    let rmpv::Value::Map(m2) = z2 else {
        panic!("expected map");
    };
    let (_, inner2) = &m2[0];
    let rmpv::Value::Array(za2) = inner2 else {
        panic!("ZoneChanged AsteroidBase: {inner2:?}");
    };
    assert_eq!(za2.len(), 1);
    assert!(matches!(&za2[0], rmpv::Value::Map(_)));

    let pd = decode_value(
        &codec::encode(&ServerMessage::PlayerDied {
            entity_id: EntityId(99),
        })
        .unwrap(),
    );
    let rmpv::Value::Map(m3) = pd else {
        panic!("expected map");
    };
    let (_, inner3) = &m3[0];
    let rmpv::Value::Array(a) = inner3 else {
        panic!("PlayerDied inner: {inner3:?}");
    };
    assert_eq!(a.len(), 1);

    let de =
        decode_value(&codec::encode(&ServerMessage::DynamicEventEnded { event_id: 7 }).unwrap());
    let rmpv::Value::Map(m4) = de else {
        panic!("expected map");
    };
    let (_, inner4) = &m4[0];
    let rmpv::Value::Array(a4) = inner4 else {
        panic!("DynamicEventEnded inner: {inner4:?}");
    };
    assert_eq!(a4.len(), 1);
}
