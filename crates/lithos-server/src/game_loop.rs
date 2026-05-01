//! The main game loop — ties together networking, ECS simulation, and broadcasting.

use anyhow::{Context, Result};
use bytes::Bytes;
use rand::Rng;
use serde::Serialize;
use sqlx::Row;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use lithos_protocol::{
    ChatChannel, ClientMessage, DynamicEventSnapshot, EntityId, EntitySnapshot, InteractableKind,
    InteractableSnapshot, RaidStateSnapshot, ServerMessage, SkillBranch, Vec2, ZoneId, codec,
};
use lithos_world::components::{
    BaseTile, BossPhase, Collider, CommsArray, DroneBay as DroneBayComponent, FabricationPlant,
    Flying, HackingTarget, Health, Hydroponics, Inventory, Item, LastLoadoutTick, Npc, NpcState,
    NpcType, Oxygen, Player, Position, PositionHistory, PowerConsumer, PowerGenerator, Progression,
    ResourceNode, ResourceType, SalvageSite, TileType, Velocity, Weapon, Zone,
};
use lithos_world::resources::{
    ChatEvent, ChatEvents, EntityRegistry, FactionVaults, FireRequest, InputQueue,
    LastProcessedSeq, MoveInput, ProgressionQueue, RaidState, RaidStateStore, RespawnRequest,
    SimConfig, TraderMarket, XpGainRequest, ZoneChangeEvents, ZoneTransferRequest,
};
use lithos_world::simulation::Simulation;
use lithos_world::tilemap::{ChunkCoord, TileMap};
use lithos_world::world_gen::{Biome, WorldGenerator};

use crate::ServerConfig;
use crate::auth;
use crate::connection::ConnectionManager;
use crate::network::{self, NetworkEvent};

#[derive(Debug, Clone)]
struct AuthJoin {
    auth_subject: Option<String>,
    username: String,
    faction_id: Option<u64>,
}

#[derive(Debug, Serialize)]
struct HeartbeatPayload<'a> {
    server_id: &'a str,
    name: &'a str,
    websocket_url: &'a str,
    region: &'a str,
    population: u32,
    capacity: u32,
    healthy: bool,
    world_seed: u32,
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn normalize_username(name: &str) -> String {
    let mut cleaned = String::with_capacity(name.len().min(24));
    for ch in name.chars() {
        if cleaned.len() >= 24 {
            break;
        }
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            cleaned.push(ch);
        }
    }
    if cleaned.is_empty() {
        "guest".to_string()
    } else {
        cleaned
    }
}

async fn resolve_join_from_token(token: &str, config: &ServerConfig) -> Result<AuthJoin> {
    let token = token.trim();
    if let Some(jwks_url) = config.supabase_jwks_url.as_deref() {
        let claims = auth::validate_supabase_jwt(
            token,
            jwks_url,
            config.supabase_jwt_issuer.as_deref(),
            config.supabase_jwt_audience.as_deref(),
        )
        .await
        .context("supabase JWT validation failed")?;

        let username = claims
            .preferred_username
            .clone()
            .or_else(|| {
                claims
                    .email
                    .clone()
                    .and_then(|email| email.split('@').next().map(ToOwned::to_owned))
            })
            .unwrap_or_else(|| {
                let short = claims.sub.chars().take(8).collect::<String>();
                format!("pilot-{short}")
            });

        return Ok(AuthJoin {
            auth_subject: Some(claims.sub),
            username: normalize_username(&username),
            faction_id: None,
        });
    }

    // Development fallback when JWT validation is not configured.
    // Supported format: "username#faction_id".
    let (name_part, faction_id) = if let Some((name, faction_raw)) = token.split_once('#') {
        (name, faction_raw.parse::<u64>().ok())
    } else {
        (token, None)
    };

    let username = if name_part.is_empty() {
        format!("guest-{}", uuid::Uuid::new_v4().simple())
    } else {
        normalize_username(name_part)
    };

    Ok(AuthJoin {
        auth_subject: None,
        username,
        faction_id,
    })
}

fn encode_and_send(outbound: &mpsc::UnboundedSender<Bytes>, msg: &ServerMessage) {
    if let Ok(bytes) = codec::encode(msg) {
        let _ = outbound.send(Bytes::from(bytes));
    }
}

fn send_to_entity(connections: &ConnectionManager, entity_id: EntityId, msg: &ServerMessage) {
    if let Some(conn) = connections.get(entity_id) {
        encode_and_send(&conn.outbound_tx, msg);
    }
}

fn broadcast_all(connections: &ConnectionManager, msg: &ServerMessage) {
    if let Ok(vec) = codec::encode(msg) {
        let payload = Bytes::from(vec);
        for conn in connections.iter() {
            let _ = conn.outbound_tx.send(payload.clone());
        }
    }
}

fn send_to_faction(connections: &ConnectionManager, faction_id: u64, msg: &ServerMessage) {
    if let Ok(vec) = codec::encode(msg) {
        let payload = Bytes::from(vec);
        for conn in connections.iter() {
            if conn.faction_id == Some(faction_id) {
                let _ = conn.outbound_tx.send(payload.clone());
            }
        }
    }
}

fn raid_snapshot(raid: &RaidState, tick: u64, dt: f32) -> RaidStateSnapshot {
    let warning_ticks = if raid.breach_active {
        0
    } else {
        raid.warning_ends_at_tick.saturating_sub(tick)
    };
    RaidStateSnapshot {
        attacker_faction_id: raid.attacker_faction_id,
        defender_faction_id: raid.defender_faction_id,
        warning_remaining_seconds: (warning_ticks as f32 * dt).ceil() as u32,
        breach_active: raid.breach_active,
    }
}

fn seed_world(sim: &mut Simulation, world_seed: u32) {
    use lithos_world::components::{CommsArray, FabricationPlant, HackingTarget, SalvageSite};
    use lithos_world::tilemap::{CHUNK_WORLD_SIZE, TerrainType as TileTerrain};

    let mut rng = rand::thread_rng();
    let generator = WorldGenerator::new(world_seed);

    // Pre-generate chunks covering the full world area so we can query terrain.
    let world_half = 2000.0;
    let max_chunk = (world_half / CHUNK_WORLD_SIZE).ceil() as i32;
    {
        let mut tilemap = sim.world.resource_mut::<TileMap>();
        for cy in -max_chunk..=max_chunk {
            for cx in -max_chunk..=max_chunk {
                tilemap.ensure_chunk(ChunkCoord { x: cx, y: cy });
            }
        }
    }

    // ── Hostiles ─────────────────────────────────────────────────────────────
    for _ in 0..160 {
        let pos = Vec2::new(
            rng.gen_range(-world_half..world_half),
            rng.gen_range(-world_half..world_half),
        );
        let biome = generator.get_biome(pos);

        let passable = {
            let tilemap = sim.world.resource::<TileMap>();
            tilemap
                .get_tile_loaded(pos)
                .map(|t| t.is_ground_passable())
                .unwrap_or(false)
        };
        if !passable {
            continue;
        }

        // Determine enemy type and stats based on biome.
        let (npc_type, health, weapon, collider_radius, flying) = match biome {
            Biome::OuterRim => (
                NpcType::Rover,
                60.0,
                Weapon {
                    damage: 20.0,
                    projectile_speed: 300.0,
                    cooldown_seconds: 0.8,
                    last_fired_time: 0.0,
                    ammo: 100,
                    max_ammo: 100,
                },
                10.0,
                false,
            ),
            Biome::MidZone => {
                if rng.gen_bool(0.5) {
                    (
                        NpcType::Drone,
                        80.0,
                        Weapon {
                            damage: 10.0,
                            projectile_speed: 500.0,
                            cooldown_seconds: 0.3,
                            last_fired_time: 0.0,
                            ammo: 200,
                            max_ammo: 200,
                        },
                        10.0,
                        true,
                    )
                } else {
                    (
                        NpcType::AssaultWalker,
                        150.0,
                        Weapon {
                            damage: 25.0,
                            projectile_speed: 400.0,
                            cooldown_seconds: 0.6,
                            last_fired_time: 0.0,
                            ammo: 60,
                            max_ammo: 60,
                        },
                        16.0,
                        false,
                    )
                }
            }
            Biome::Core => {
                let roll = rng.gen_range(0.0..1.0);
                if roll < 0.35 {
                    (
                        NpcType::AssaultWalker,
                        250.0,
                        Weapon {
                            damage: 35.0,
                            projectile_speed: 400.0,
                            cooldown_seconds: 0.5,
                            last_fired_time: 0.0,
                            ammo: 80,
                            max_ammo: 80,
                        },
                        16.0,
                        false,
                    )
                } else if roll < 0.6 {
                    (
                        NpcType::SniperWalker,
                        120.0,
                        Weapon {
                            damage: 60.0,
                            projectile_speed: 900.0,
                            cooldown_seconds: 2.0,
                            last_fired_time: 0.0,
                            ammo: 20,
                            max_ammo: 20,
                        },
                        14.0,
                        false,
                    )
                } else if roll < 0.8 {
                    (
                        NpcType::HeavyFlamethrower,
                        400.0,
                        Weapon {
                            damage: 8.0,
                            projectile_speed: 200.0,
                            cooldown_seconds: 0.1,
                            last_fired_time: 0.0,
                            ammo: 500,
                            max_ammo: 500,
                        },
                        18.0,
                        false,
                    )
                } else {
                    (
                        NpcType::Drone,
                        100.0,
                        Weapon {
                            damage: 15.0,
                            projectile_speed: 500.0,
                            cooldown_seconds: 0.3,
                            last_fired_time: 0.0,
                            ammo: 200,
                            max_ammo: 200,
                        },
                        10.0,
                        true,
                    )
                }
            }
        };

        let npc_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
        let mut ent = sim.world.spawn((
            Position(pos),
            Velocity(Vec2::ZERO),
            Zone(ZoneId::Overworld),
            Npc {
                npc_type,
                state: NpcState::Patrol,
                target: None,
                spawn_pos: pos,
                state_entered_tick: 0,
            },
            Health {
                current: health,
                max: health,
            },
            weapon,
            Collider {
                radius: collider_radius,
            },
            Inventory {
                items: vec!["scrap".to_string(), "circuit".to_string()],
            },
        ));
        if flying {
            ent.insert(Flying);
        }
        let ecs_ent = ent.id();
        sim.world
            .resource_mut::<EntityRegistry>()
            .register(npc_id, ecs_ent);
    }

    // ── Core Warden (boss) ───────────────────────────────────────────────────
    {
        let warden_pos = Vec2::new(0.0, 0.0);
        let warden_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
        let warden_ent = sim
            .world
            .spawn((
                Position(warden_pos),
                Velocity(Vec2::ZERO),
                Zone(ZoneId::Overworld),
                Npc {
                    npc_type: NpcType::CoreWarden,
                    state: NpcState::Patrol,
                    target: None,
                    spawn_pos: warden_pos,
                    state_entered_tick: 0,
                },
                Health {
                    current: 5000.0,
                    max: 5000.0,
                },
                Weapon {
                    damage: 80.0,
                    projectile_speed: 250.0,
                    cooldown_seconds: 1.5,
                    last_fired_time: 0.0,
                    ammo: 1000,
                    max_ammo: 1000,
                },
                Collider { radius: 40.0 },
                Inventory {
                    items: vec![
                        "logic_core".to_string(),
                        "plasma_cell".to_string(),
                        "high_tier_component".to_string(),
                    ],
                },
                BossPhase {
                    phase: 1,
                    last_add_spawn_tick: 0,
                },
            ))
            .id();
        sim.world
            .resource_mut::<EntityRegistry>()
            .register(warden_id, warden_ent);
    }

    // ── Traders ──────────────────────────────────────────────────────────────
    for _ in 0..20 {
        let pos = Vec2::new(
            rng.gen_range(-world_half..world_half),
            rng.gen_range(-world_half..world_half),
        );
        let biome = generator.get_biome(pos);
        if biome == Biome::Core {
            continue;
        }

        let passable = {
            let tilemap = sim.world.resource::<TileMap>();
            tilemap
                .get_tile_loaded(pos)
                .map(|t| t.is_ground_passable())
                .unwrap_or(false)
        };
        if !passable {
            continue;
        }

        let npc_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
        let ecs_ent = sim
            .world
            .spawn((
                Position(pos),
                Velocity(Vec2::ZERO),
                Zone(ZoneId::Overworld),
                Npc {
                    npc_type: NpcType::Trader,
                    state: NpcState::Patrol,
                    target: None,
                    spawn_pos: pos,
                    state_entered_tick: 0,
                },
                Health {
                    current: 500.0,
                    max: 500.0,
                },
                Collider { radius: 14.0 },
                Inventory {
                    items: vec!["medkit".to_string(), "battery".to_string()],
                },
            ))
            .id();
        sim.world
            .resource_mut::<EntityRegistry>()
            .register(npc_id, ecs_ent);
    }

    // ── Resource Nodes (terrain-aware vein spawning) ──────────────────────────
    let mut next_vein_id: u32 = 1;

    spawn_resource_veins(
        sim,
        &mut rng,
        &generator,
        ResourceType::Iron,
        80,
        8..15,
        |tile: &lithos_world::tilemap::Tile| {
            matches!(tile.terrain, TileTerrain::Rock | TileTerrain::DeepRavine)
        },
        |biome| matches!(biome, Biome::OuterRim | Biome::MidZone),
        &mut next_vein_id,
    );

    spawn_resource_veins(
        sim,
        &mut rng,
        &generator,
        ResourceType::Copper,
        60,
        6..12,
        |tile: &lithos_world::tilemap::Tile| matches!(tile.terrain, TileTerrain::Rock),
        |biome| matches!(biome, Biome::OuterRim | Biome::MidZone),
        &mut next_vein_id,
    );

    spawn_resource_veins(
        sim,
        &mut rng,
        &generator,
        ResourceType::Titanium,
        45,
        6..10,
        |tile: &lithos_world::tilemap::Tile| {
            matches!(tile.terrain, TileTerrain::Rock | TileTerrain::AsteroidField)
        },
        |biome| matches!(biome, Biome::MidZone),
        &mut next_vein_id,
    );

    spawn_resource_veins(
        sim,
        &mut rng,
        &generator,
        ResourceType::Silica,
        50,
        6..12,
        |tile: &lithos_world::tilemap::Tile| matches!(tile.terrain, TileTerrain::AsteroidField),
        |biome| matches!(biome, Biome::MidZone),
        &mut next_vein_id,
    );

    spawn_resource_veins(
        sim,
        &mut rng,
        &generator,
        ResourceType::Uranium,
        30,
        5..10,
        |tile: &lithos_world::tilemap::Tile| {
            matches!(
                tile.terrain,
                TileTerrain::DeepRavine | TileTerrain::AutomataSpire
            )
        },
        |biome| matches!(biome, Biome::MidZone | Biome::Core),
        &mut next_vein_id,
    );

    spawn_resource_veins(
        sim,
        &mut rng,
        &generator,
        ResourceType::Plutonium,
        20,
        4..8,
        |tile: &lithos_world::tilemap::Tile| matches!(tile.terrain, TileTerrain::AutomataSpire),
        |biome| matches!(biome, Biome::Core),
        &mut next_vein_id,
    );

    spawn_resource_veins(
        sim,
        &mut rng,
        &generator,
        ResourceType::BioMass,
        60,
        6..12,
        |tile: &lithos_world::tilemap::Tile| matches!(tile.terrain, TileTerrain::Empty),
        |biome| matches!(biome, Biome::OuterRim | Biome::MidZone),
        &mut next_vein_id,
    );

    // ── Salvage Sites ────────────────────────────────────────────────────────
    for _ in 0..40 {
        let pos = Vec2::new(
            rng.gen_range(-world_half..world_half),
            rng.gen_range(-world_half..world_half),
        );
        let biome = generator.get_biome(pos);
        if biome == Biome::OuterRim {
            continue;
        }

        let terrain_ok = {
            let tilemap = sim.world.resource::<TileMap>();
            tilemap
                .get_tile_loaded(pos)
                .map(|t| matches!(t.terrain, TileTerrain::Empty | TileTerrain::AsteroidField))
                .unwrap_or(false)
        };
        if !terrain_ok {
            continue;
        }

        let salvage_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
        let item_type = if rng.gen_bool(0.5) {
            "rusted_husk"
        } else {
            "abandoned_mech"
        };
        let ecs_ent = sim
            .world
            .spawn((
                Position(pos),
                Zone(ZoneId::Overworld),
                Collider { radius: 20.0 },
                SalvageSite {
                    item_type: item_type.to_string(),
                    yield_remaining: rng.gen_range(3..8),
                    required_tool: "salvage_torch".to_string(),
                },
            ))
            .id();
        sim.world
            .resource_mut::<EntityRegistry>()
            .register(salvage_id, ecs_ent);
    }

    // ── POIs ─────────────────────────────────────────────────────────────────
    // Fabrication Plants (2–3 in Mid-Zone, 1 in Core)
    let mut plant_count = 0;
    let target_plants = rng.gen_range(3..=4);
    while plant_count < target_plants {
        let pos = Vec2::new(
            rng.gen_range(-world_half..world_half),
            rng.gen_range(-world_half..world_half),
        );
        let biome = generator.get_biome(pos);
        if biome == Biome::OuterRim {
            continue;
        }

        let terrain_ok = {
            let tilemap = sim.world.resource::<TileMap>();
            tilemap
                .get_tile_loaded(pos)
                .map(|t| t.is_ground_passable())
                .unwrap_or(false)
        };
        if !terrain_ok {
            continue;
        }

        let poi_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
        let ecs_ent = sim
            .world
            .spawn((
                Position(pos),
                Zone(ZoneId::Overworld),
                Collider { radius: 30.0 },
                FabricationPlant { tier_bonus: 1 },
            ))
            .id();
        sim.world
            .resource_mut::<EntityRegistry>()
            .register(poi_id, ecs_ent);
        plant_count += 1;
    }

    // Comms Arrays (3–5 across map)
    let mut array_count = 0;
    let target_arrays = rng.gen_range(3..=5);
    while array_count < target_arrays {
        let pos = Vec2::new(
            rng.gen_range(-world_half..world_half),
            rng.gen_range(-world_half..world_half),
        );

        let terrain_ok = {
            let tilemap = sim.world.resource::<TileMap>();
            tilemap
                .get_tile_loaded(pos)
                .map(|t| t.is_ground_passable() && t.height > 150)
                .unwrap_or(false)
        };
        if !terrain_ok {
            continue;
        }

        let poi_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
        let ecs_ent = sim
            .world
            .spawn((
                Position(pos),
                Zone(ZoneId::Overworld),
                Collider { radius: 20.0 },
                CommsArray {
                    hackable: true,
                    reveals_minimap: true,
                },
                HackingTarget {
                    hack_time_seconds: 6.0,
                    reward_table_id: "comms_array".to_string(),
                    is_hacked: false,
                },
            ))
            .id();
        sim.world
            .resource_mut::<EntityRegistry>()
            .register(poi_id, ecs_ent);
        array_count += 1;
    }
}

/// Helper: spawn resource veins at terrain-matching locations.
#[allow(clippy::too_many_arguments)]
fn spawn_resource_veins(
    sim: &mut Simulation,
    rng: &mut impl rand::Rng,
    generator: &WorldGenerator,
    resource_type: ResourceType,
    vein_count: usize,
    yield_range: std::ops::Range<u32>,
    terrain_filter: impl Fn(&lithos_world::tilemap::Tile) -> bool,
    biome_filter: impl Fn(Biome) -> bool,
    next_vein_id: &mut u32,
) {
    let world_half = 2000.0;
    let mut attempts = 0;
    let mut spawned = 0;

    while spawned < vein_count && attempts < vein_count * 50 {
        attempts += 1;
        let center = Vec2::new(
            rng.gen_range(-world_half..world_half),
            rng.gen_range(-world_half..world_half),
        );
        let biome = generator.get_biome(center);
        if !biome_filter(biome) {
            continue;
        }

        let center_ok = {
            let tilemap = sim.world.resource::<TileMap>();
            tilemap
                .get_tile_loaded(center)
                .map(&terrain_filter)
                .unwrap_or(false)
        };
        if !center_ok {
            continue;
        }

        // Spawn a vein: cluster of nodes around the center.
        let vein_id = *next_vein_id;
        *next_vein_id += 1;
        let cluster_size = rng.gen_range(8..15);

        for _ in 0..cluster_size {
            let offset = Vec2::new(rng.gen_range(-120.0..120.0), rng.gen_range(-120.0..120.0));
            let pos = center + offset;

            let pos_ok = {
                let tilemap = sim.world.resource::<TileMap>();
                tilemap
                    .get_tile_loaded(pos)
                    .map(&terrain_filter)
                    .unwrap_or(false)
            };
            if !pos_ok {
                continue;
            }

            let node_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
            let ecs_ent = sim
                .world
                .spawn((
                    Position(pos),
                    Zone(ZoneId::Overworld),
                    Collider { radius: 16.0 },
                    ResourceNode {
                        resource_type: resource_type.clone(),
                        yield_amount: rng.gen_range(yield_range.clone()),
                        vein_id: Some(vein_id),
                    },
                ))
                .id();
            sim.world
                .resource_mut::<EntityRegistry>()
                .register(node_id, ecs_ent);
        }
        spawned += 1;
    }
}

async fn report_heartbeat(config: &ServerConfig, population: usize) {
    let url = format!(
        "{}/internal/servers/heartbeat",
        config.central_api_url.trim_end_matches('/')
    );
    let payload = HeartbeatPayload {
        server_id: &config.server_id,
        name: &config.server_name,
        websocket_url: &config.websocket_public_url,
        region: &config.region,
        population: population as u32,
        capacity: config.max_players as u32,
        healthy: true,
        world_seed: config.world_seed,
    };

    let client = reqwest::Client::new();
    let mut req = client.post(url).json(&payload);
    if let Some(key) = config.central_api_key.as_deref() {
        req = req.header("x-api-key", key);
    }

    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            tracing::debug!(population, "heartbeat sent");
        }
        Ok(resp) => {
            tracing::warn!(status = %resp.status(), "heartbeat rejected by central api");
        }
        Err(error) => {
            tracing::warn!(?error, "failed to send heartbeat");
        }
    }
}

/// Run the game server.
pub async fn run(config: ServerConfig, pool: sqlx::PgPool) -> Result<()> {
    let listener = TcpListener::bind(&config.listen_addr).await?;
    tracing::info!(addr = %config.listen_addr, "WebSocket listener ready");

    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<NetworkEvent>();

    let accept_event_tx = event_tx.clone();
    tokio::spawn(async move {
        let mut next_id: u64 = 1_000_000;
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let entity_id = EntityId(next_id);
                    next_id += 1;
                    let tx = accept_event_tx.clone();
                    tokio::spawn(network::handle_connection(stream, entity_id, tx));
                }
                Err(error) => {
                    tracing::error!(?error, "accept error");
                }
            }
        }
    });

    let tick_duration = Duration::from_secs_f64(1.0 / config.tick_rate as f64);
    let sim_config = lithos_world::resources::SimConfig {
        dt: tick_duration.as_secs_f32(),
        max_speed: 200.0,
        world_half_size: 2000.0,
        lag_comp_history_ticks: 64,
        world_seed: config.world_seed,
    };
    let mut sim = Simulation::with_config(sim_config);
    let mut connections = ConnectionManager::new();
    let mut unauth_connections = HashMap::new();

    seed_world(&mut sim, config.world_seed);
    tracing::info!(tick_rate = config.tick_rate, "game loop starting");

    let mut flush_counter = 0_u64;
    let mut heartbeat_counter = 0_u64;
    let mut player_loaded_chunks: HashMap<EntityId, std::collections::HashSet<ChunkCoord>> =
        HashMap::new();

    loop {
        let tick_start = Instant::now();

        while let Ok(event) = event_rx.try_recv() {
            if let Err(error) = handle_event(
                event,
                &mut sim,
                &mut connections,
                &mut unauth_connections,
                &config,
                &pool,
            )
            .await
            {
                tracing::warn!(?error, "failed to process network event");
            }
        }

        sim.tick();

        send_zone_changes(&mut sim, &connections);
        send_combat_events(&mut sim, &connections);
        send_mining_events(&mut sim, &connections);
        send_trade_events(&mut sim, &connections);
        send_chat_events(&mut sim, &connections);
        send_dynamic_events(&mut sim, &connections);
        send_raid_events(&mut sim, &connections);
        broadcast_snapshots(&mut sim, &connections);

        // Chunk loading: ensure players have the terrain around them.
        let chunk_radius = 3;
        load_and_send_chunks(
            &mut sim,
            &connections,
            chunk_radius,
            &mut player_loaded_chunks,
        )
        .await;

        flush_counter += 1;
        if flush_counter.is_multiple_of(600) {
            flush_player_states(&sim, &connections, &pool).await;
        }

        heartbeat_counter += 1;
        if heartbeat_counter.is_multiple_of(100) {
            report_heartbeat(&config, connections.count()).await;
        }

        let elapsed = tick_start.elapsed();
        if elapsed >= tick_duration {
            tracing::warn!(
                elapsed_ms = elapsed.as_millis(),
                budget_ms = tick_duration.as_millis(),
                "tick overran budget"
            );
        }

        if elapsed < tick_duration {
            tokio::time::sleep(tick_duration - elapsed).await;
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_event(
    event: NetworkEvent,
    sim: &mut Simulation,
    connections: &mut ConnectionManager,
    unauth_connections: &mut HashMap<EntityId, mpsc::UnboundedSender<Bytes>>,
    config: &ServerConfig,
    pool: &sqlx::PgPool,
) -> Result<()> {
    match event {
        NetworkEvent::Connected {
            entity_id,
            outbound_tx,
        } => {
            unauth_connections.insert(entity_id, outbound_tx);
        }
        NetworkEvent::Message { entity_id, message } => match message {
            ClientMessage::Join { token } => {
                let Some(outbound_tx) = unauth_connections.remove(&entity_id) else {
                    return Ok(());
                };

                if connections.count() >= config.max_players {
                    encode_and_send(
                        &outbound_tx,
                        &ServerMessage::Disconnect {
                            reason: "server full".to_string(),
                        },
                    );
                    return Ok(());
                }

                let auth = match resolve_join_from_token(&token, config).await {
                    Ok(join) => join,
                    Err(error) => {
                        tracing::warn!(?error, "join rejected");
                        encode_and_send(
                            &outbound_tx,
                            &ServerMessage::Disconnect {
                                reason: "invalid token".to_string(),
                            },
                        );
                        return Ok(());
                    }
                };

                let row =
                    sqlx::query("SELECT x, y, health, faction_id FROM players WHERE username = $1")
                        .bind(&auth.username)
                        .fetch_optional(pool)
                        .await?;

                let pos = row
                    .as_ref()
                    .map(|r| {
                        Vec2::new(
                            r.try_get::<f64, _>("x").unwrap_or(0.0) as f32,
                            r.try_get::<f64, _>("y").unwrap_or(0.0) as f32,
                        )
                    })
                    .unwrap_or(Vec2::ZERO);
                let health = row
                    .as_ref()
                    .map(|r| r.try_get::<f64, _>("health").unwrap_or(100.0) as f32)
                    .unwrap_or(100.0);
                let persisted_faction = row
                    .as_ref()
                    .and_then(|r| r.try_get::<i64, _>("faction_id").ok())
                    .map(|v| v as u64);
                let faction_id = auth.faction_id.or(persisted_faction);

                if row.is_none() {
                    sqlx::query(
                        "INSERT INTO players (id, username, x, y, zone_id, health, inventory, auth_subject, faction_id) \
                         VALUES ($1, $2, 0.0, 0.0, 'overworld', 100.0, '[]', $3, $4)",
                    )
                    .bind(uuid::Uuid::new_v4())
                    .bind(&auth.username)
                    .bind(auth.auth_subject.as_deref())
                    .bind(faction_id.map(|id| id as i64))
                    .execute(pool)
                    .await?;
                } else {
                    sqlx::query(
                        "UPDATE players SET auth_subject = COALESCE($1, auth_subject), faction_id = COALESCE($2, faction_id) WHERE username = $3",
                    )
                    .bind(auth.auth_subject.as_deref())
                    .bind(faction_id.map(|id| id as i64))
                    .bind(&auth.username)
                    .execute(pool)
                    .await?;
                }

                let player_id = lithos_protocol::PlayerId::new();
                let current_tick = sim.current_tick();
                let ecs_entity = sim
                    .world
                    .spawn((
                        Position(pos),
                        Velocity(Vec2::ZERO),
                        PositionHistory::default(),
                        Progression::default(),
                        Player {
                            id: player_id,
                            auth_subject: auth.auth_subject.clone(),
                            faction_id,
                        },
                        Zone(ZoneId::Overworld),
                        Health {
                            current: health,
                            max: 100.0,
                        },
                        Weapon {
                            damage: 20.0,
                            projectile_speed: 600.0,
                            cooldown_seconds: 0.5,
                            last_fired_time: 0.0,
                            ammo: 50,
                            max_ammo: 50,
                        },
                        Collider { radius: 14.0 },
                        Inventory {
                            items: vec![
                                "mining_laser".to_string(),
                                "scrap".to_string(),
                                "scrap".to_string(),
                            ],
                        },
                        Oxygen {
                            current: 100.0,
                            max: 100.0,
                        },
                        LastLoadoutTick { tick: current_tick },
                    ))
                    .id();

                let mut registry = sim.world.resource_mut::<EntityRegistry>();
                registry.register(entity_id, ecs_entity);
                registry.player_entities.insert(player_id, entity_id);

                if let Some(faction_id) = faction_id {
                    let mut vaults = sim.world.resource_mut::<FactionVaults>();
                    vaults.balances.entry(faction_id).or_insert(1_000);
                }

                connections.add(
                    player_id,
                    entity_id,
                    faction_id,
                    auth.username.clone(),
                    outbound_tx.clone(),
                );

                encode_and_send(
                    &outbound_tx,
                    &ServerMessage::JoinAck {
                        player_id,
                        entity_id,
                        zone: ZoneId::Overworld,
                        world_seed: config.world_seed,
                    },
                );

                // Sync initial inventory and ammo so the HUD is correct from the first frame.
                let entity_ref = sim.world.entity(ecs_entity);
                let items_json = entity_ref
                    .get::<Inventory>()
                    .map(|i| serde_json::to_string(&i.items).unwrap_or_else(|_| "[]".to_string()))
                    .unwrap_or_else(|| "[]".to_string());
                encode_and_send(
                    &outbound_tx,
                    &ServerMessage::InventoryUpdated {
                        entity_id,
                        items_json,
                    },
                );

                if let Some(weapon) = entity_ref.get::<Weapon>() {
                    encode_and_send(
                        &outbound_tx,
                        &ServerMessage::AmmoChanged {
                            entity_id,
                            ammo: weapon.ammo,
                            max_ammo: weapon.max_ammo,
                        },
                    );
                }
            }
            ClientMessage::Move { direction, seq } => {
                sim.world
                    .resource_mut::<InputQueue>()
                    .moves
                    .push(MoveInput {
                        entity_id,
                        direction,
                        seq,
                    });
            }
            ClientMessage::ZoneTransfer { target } => {
                let mut resolved_target = target;

                // Redirect AsteroidBase requests to faction-specific base.
                if matches!(target, ZoneId::AsteroidBase(_))
                    && let Some(conn) = connections.get(entity_id)
                {
                    if let Some(faction_id) = conn.faction_id {
                        resolved_target = ZoneId::AsteroidBase(faction_id as u32);
                    } else {
                        send_to_entity(
                            connections,
                            entity_id,
                            &ServerMessage::ChatMessage {
                                from_entity_id: lithos_protocol::EntityId(0),
                                channel: ChatChannel::Global,
                                text: "You need a faction to enter an Asteroid Base.".to_string(),
                                sent_at_unix_ms: now_unix_ms(),
                            },
                        );
                        return Ok(());
                    }
                }

                // Load base structures from DB if entering an AsteroidBase for the first time.
                if let ZoneId::AsteroidBase(base_faction_id) = resolved_target {
                    let already_loaded = sim
                        .world
                        .resource::<lithos_world::resources::LoadedZones>()
                        .zones
                        .contains(&resolved_target);

                    if !already_loaded {
                        sim.world
                            .resource_mut::<lithos_world::resources::LoadedZones>()
                            .zones
                            .insert(resolved_target);

                        let zone_str = format!("asteroid_{base_faction_id}");
                        let rows = sqlx::query(
                            "SELECT tile_type, grid_x, grid_y FROM base_structures WHERE zone_id = $1",
                        )
                        .bind(&zone_str)
                        .fetch_all(pool)
                        .await?;

                        for row in rows {
                            let tile_type_str: String = row.try_get("tile_type")?;
                            let grid_x: i32 = row.try_get("grid_x")?;
                            let grid_y: i32 = row.try_get("grid_y")?;

                            let tile_type = match tile_type_str.as_str() {
                                "wall_segment" => Some(TileType::Wall),
                                "door" => Some(TileType::Door),
                                "workbench" => Some(TileType::Workbench),
                                "generator" => Some(TileType::Generator),
                                "hydroponics_tray" => Some(TileType::HydroponicsTray),
                                "drone_bay" => Some(TileType::DroneBay),
                                _ => None,
                            };

                            if let Some(tt) = tile_type {
                                let world_pos =
                                    Vec2::new(grid_x as f32 * 40.0, grid_y as f32 * 40.0);
                                let id =
                                    sim.world.resource_mut::<EntityRegistry>().next_entity_id();
                                let mut entity = sim.world.spawn((
                                    Position(world_pos),
                                    Zone(resolved_target),
                                    BaseTile {
                                        tile_type: tt.clone(),
                                        grid_x,
                                        grid_y,
                                    },
                                    Collider { radius: 20.0 },
                                ));

                                match tt {
                                    TileType::Generator => {
                                        entity.insert(PowerGenerator {
                                            output_kw: 100.0,
                                            fuel_remaining: 99_999.0,
                                        });
                                    }
                                    TileType::Door
                                    | TileType::Workbench
                                    | TileType::HydroponicsTray
                                    | TileType::DroneBay => {
                                        entity.insert(PowerConsumer {
                                            required_kw: 10.0,
                                            is_powered: false,
                                        });
                                        if tt == TileType::HydroponicsTray {
                                            entity.insert(Hydroponics {
                                                growth: 0.0,
                                                powered_growth_per_tick: 0.3,
                                            });
                                        }
                                        if tt == TileType::DroneBay {
                                            entity.insert(DroneBayComponent { active_drones: 0 });
                                        }
                                    }
                                    TileType::Wall => {}
                                }
                                let ecs_id = entity.id();
                                sim.world
                                    .resource_mut::<EntityRegistry>()
                                    .register(id, ecs_id);
                            }
                        }
                    }
                }

                sim.world
                    .resource_mut::<InputQueue>()
                    .zone_transfers
                    .push(ZoneTransferRequest {
                        entity_id,
                        target: resolved_target,
                    });
            }
            ClientMessage::Fire {
                direction,
                client_latency_ms,
            } => {
                sim.world
                    .resource_mut::<InputQueue>()
                    .fires
                    .push(FireRequest {
                        entity_id,
                        direction,
                        client_latency_ms,
                    });
                sim.world
                    .resource_mut::<ProgressionQueue>()
                    .gains
                    .push(XpGainRequest {
                        entity_id,
                        branch: SkillBranch::Ballistics,
                        amount: 2,
                    });
            }
            ClientMessage::Respawn => {
                sim.world
                    .resource_mut::<InputQueue>()
                    .respawns
                    .push(RespawnRequest { entity_id });
            }
            ClientMessage::Mine { target_entity_id } => {
                sim.world
                    .resource_mut::<lithos_world::resources::MineQueue>()
                    .requests
                    .push(lithos_world::resources::MineRequest {
                        entity_id,
                        target_entity_id,
                    });
            }
            ClientMessage::Ping { timestamp } => {
                send_to_entity(
                    connections,
                    entity_id,
                    &ServerMessage::Pong {
                        client_timestamp: timestamp,
                        server_timestamp: now_unix_ms(),
                    },
                );
            }
            ClientMessage::Craft { recipe } => {
                let Some(&ecs_entity) =
                    sim.world.resource::<EntityRegistry>().by_id.get(&entity_id)
                else {
                    return Ok(());
                };

                let Some(recipe_def) = lithos_world::crafting::RECIPES
                    .iter()
                    .find(|candidate| candidate.name == recipe)
                else {
                    return Ok(());
                };

                // Check skill level requirement.
                let has_level = {
                    let entity_ref = sim.world.entity(ecs_entity);
                    if let Some(progression) = entity_ref.get::<Progression>() {
                        progression
                            .branches
                            .get(&recipe_def.required_branch)
                            .map(|b| b.level >= recipe_def.required_level)
                            .unwrap_or(false)
                    } else {
                        false
                    }
                };

                if !has_level {
                    send_to_entity(
                        connections,
                        entity_id,
                        &ServerMessage::CraftDenied {
                            reason: format!(
                                "requires {:?} level {}",
                                recipe_def.required_branch, recipe_def.required_level
                            ),
                        },
                    );
                    return Ok(());
                }

                let inv_items = {
                    let entity_ref = sim.world.entity(ecs_entity);
                    let Some(inv) = entity_ref.get::<Inventory>() else {
                        return Ok(());
                    };
                    inv.items.clone()
                };

                let mut temp_inv = inv_items;
                let mut has_all = true;
                for ingredient in recipe_def.inputs {
                    if let Some(index) = temp_inv.iter().position(|item| item == *ingredient) {
                        temp_inv.remove(index);
                    } else {
                        has_all = false;
                        break;
                    }
                }

                if has_all {
                    temp_inv.push(recipe_def.output.to_string());
                    if let Some(mut inv) = sim.world.entity_mut(ecs_entity).get_mut::<Inventory>() {
                        inv.items = temp_inv.clone();
                    }

                    sim.world
                        .resource_mut::<ProgressionQueue>()
                        .gains
                        .push(XpGainRequest {
                            entity_id,
                            branch: SkillBranch::Fabrication,
                            amount: 10,
                        });

                    send_to_entity(
                        connections,
                        entity_id,
                        &ServerMessage::InventoryUpdated {
                            entity_id,
                            items_json: serde_json::to_string(&temp_inv)
                                .unwrap_or_else(|_| "[]".to_string()),
                        },
                    );
                }
            }
            ClientMessage::BuildStructure {
                item,
                grid_x,
                grid_y,
            } => {
                let Some(&ecs_entity) =
                    sim.world.resource::<EntityRegistry>().by_id.get(&entity_id)
                else {
                    return Ok(());
                };

                let (zone, mut inv_items) = {
                    let entity_ref = sim.world.entity(ecs_entity);
                    let zone = entity_ref
                        .get::<Zone>()
                        .map(|z| z.0)
                        .unwrap_or(ZoneId::Overworld);
                    let items = entity_ref
                        .get::<Inventory>()
                        .map(|inv| inv.items.clone())
                        .unwrap_or_default();
                    (zone, items)
                };

                let Some(index) = inv_items.iter().position(|i| i == &item) else {
                    return Ok(());
                };
                inv_items.remove(index);

                if let Some(mut inv) = sim.world.entity_mut(ecs_entity).get_mut::<Inventory>() {
                    inv.items = inv_items.clone();
                }

                let tile_type = match item.as_str() {
                    "wall_segment" => Some(TileType::Wall),
                    "door" => Some(TileType::Door),
                    "workbench" => Some(TileType::Workbench),
                    "generator" => Some(TileType::Generator),
                    "hydroponics_tray" => Some(TileType::HydroponicsTray),
                    "drone_bay" => Some(TileType::DroneBay),
                    _ => None,
                };

                if let Some(tile_type) = tile_type {
                    let world_pos = Vec2::new(grid_x as f32 * 40.0, grid_y as f32 * 40.0);
                    let id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
                    let mut entity = sim.world.spawn((
                        Position(world_pos),
                        Zone(zone),
                        BaseTile {
                            tile_type: tile_type.clone(),
                            grid_x,
                            grid_y,
                        },
                        Collider { radius: 20.0 },
                    ));
                    match tile_type {
                        TileType::Generator => {
                            entity.insert(PowerGenerator {
                                output_kw: 100.0,
                                fuel_remaining: 99_999.0,
                            });
                        }
                        TileType::Door
                        | TileType::Workbench
                        | TileType::HydroponicsTray
                        | TileType::DroneBay => {
                            entity.insert(PowerConsumer {
                                required_kw: 10.0,
                                is_powered: false,
                            });
                            if tile_type == TileType::HydroponicsTray {
                                entity.insert(Hydroponics {
                                    growth: 0.0,
                                    powered_growth_per_tick: 0.3,
                                });
                            }
                            if tile_type == TileType::DroneBay {
                                entity.insert(DroneBayComponent { active_drones: 0 });
                            }
                        }
                        TileType::Wall => {}
                    }
                    let ecs_id = entity.id();
                    sim.world
                        .resource_mut::<EntityRegistry>()
                        .register(id, ecs_id);

                    let zone_str = match zone {
                        ZoneId::Overworld => "overworld".to_string(),
                        ZoneId::AsteroidBase(id) => format!("asteroid_{id}"),
                    };

                    sqlx::query(
                        "INSERT INTO base_structures (zone_id, tile_type, grid_x, grid_y) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                    )
                    .bind(zone_str)
                    .bind(&item)
                    .bind(grid_x)
                    .bind(grid_y)
                    .execute(pool)
                    .await?;

                    sim.world
                        .resource_mut::<ProgressionQueue>()
                        .gains
                        .push(XpGainRequest {
                            entity_id,
                            branch: SkillBranch::Fabrication,
                            amount: 5,
                        });
                }

                send_to_entity(
                    connections,
                    entity_id,
                    &ServerMessage::InventoryUpdated {
                        entity_id,
                        items_json: serde_json::to_string(&inv_items)
                            .unwrap_or_else(|_| "[]".to_string()),
                    },
                );
            }
            ClientMessage::Chat { channel, text } => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return Ok(());
                }
                let clipped = if trimmed.len() > 280 {
                    trimmed[..280].to_string()
                } else {
                    trimmed.to_string()
                };
                let faction_id = connections.get(entity_id).and_then(|conn| conn.faction_id);
                sim.world
                    .resource_mut::<ChatEvents>()
                    .messages
                    .push(ChatEvent {
                        from_entity_id: entity_id,
                        channel,
                        text: clipped,
                        sent_at_unix_ms: now_unix_ms(),
                        faction_id,
                    });
            }
            ClientMessage::RequestTraderQuotes => {
                let quotes = sim
                    .world
                    .resource::<TraderMarket>()
                    .quotes
                    .iter()
                    .map(|quote| quote.as_quote())
                    .collect::<Vec<_>>();
                send_to_entity(
                    connections,
                    entity_id,
                    &ServerMessage::TraderQuotes { quotes },
                );
            }
            ClientMessage::SellItem { item, quantity } => {
                sim.world
                    .resource_mut::<lithos_world::resources::TradeQueue>()
                    .requests
                    .push(lithos_world::resources::TradeRequest {
                        entity_id,
                        item,
                        quantity,
                        is_sell: true,
                    });
            }
            ClientMessage::BuyItem { item, quantity } => {
                sim.world
                    .resource_mut::<lithos_world::resources::TradeQueue>()
                    .requests
                    .push(lithos_world::resources::TradeRequest {
                        entity_id,
                        item,
                        quantity,
                        is_sell: false,
                    });
            }
            ClientMessage::Interact { target_entity_id } => {
                let Some(target_id) = target_entity_id else {
                    return Ok(());
                };
                let Some(&player_ecs) =
                    sim.world.resource::<EntityRegistry>().by_id.get(&entity_id)
                else {
                    return Ok(());
                };
                let Some(&target_ecs) =
                    sim.world.resource::<EntityRegistry>().by_id.get(&target_id)
                else {
                    return Ok(());
                };
                let (player_pos, player_zone) = {
                    let Ok(player_ent) = sim.world.get_entity(player_ecs) else {
                        return Ok(());
                    };
                    let pos = player_ent
                        .get::<Position>()
                        .map(|position| position.0)
                        .unwrap_or(Vec2::ZERO);
                    let zone = player_ent
                        .get::<Zone>()
                        .map(|zone| zone.0)
                        .unwrap_or(ZoneId::Overworld);
                    (pos, zone)
                };
                let (target_pos, target_zone) = {
                    let Ok(target_ent) = sim.world.get_entity(target_ecs) else {
                        return Ok(());
                    };
                    let pos = target_ent
                        .get::<Position>()
                        .map(|position| position.0)
                        .unwrap_or(Vec2::ZERO);
                    let zone = target_ent
                        .get::<Zone>()
                        .map(|zone| zone.0)
                        .unwrap_or(ZoneId::Overworld);
                    (pos, zone)
                };
                if player_zone != target_zone
                    || (player_pos - target_pos).length_squared() > 220.0 * 220.0
                {
                    return Ok(());
                }
                let mut interactable = InteractableSnapshot {
                    target_entity_id: target_id,
                    kind: InteractableKind::ResourceNode,
                    required_tool: None,
                    can_interact: true,
                };

                let mut inventory_items = {
                    let Ok(player_ent) = sim.world.get_entity(player_ecs) else {
                        return Ok(());
                    };
                    player_ent
                        .get::<Inventory>()
                        .map(|inv| inv.items.clone())
                        .unwrap_or_default()
                };

                let mut consumed = false;
                if let Ok(mut target) = sim.world.get_entity_mut(target_ecs) {
                    if let Some(mut salvage) = target.get_mut::<SalvageSite>() {
                        interactable.kind = InteractableKind::SalvageSite;
                        interactable.required_tool = Some("salvage_torch".to_string());
                        let has_tool = inventory_items.iter().any(|item| item == "salvage_torch");
                        interactable.can_interact = has_tool;
                        if has_tool && salvage.yield_remaining > 0 {
                            salvage.yield_remaining = salvage.yield_remaining.saturating_sub(1);
                            inventory_items.push("gears".to_string());
                            consumed = salvage.yield_remaining == 0;
                        }
                    } else if let Some(mut hack) = target.get_mut::<HackingTarget>() {
                        interactable.kind = InteractableKind::HackingTarget;
                        interactable.required_tool = Some("hacking_tool".to_string());
                        if !hack.is_hacked {
                            hack.is_hacked = true;
                            inventory_items.push("encrypted_drive".to_string());
                            sim.world.resource_mut::<ProgressionQueue>().gains.push(
                                XpGainRequest {
                                    entity_id,
                                    branch: SkillBranch::Cybernetics,
                                    amount: 8,
                                },
                            );
                        }
                    } else if target.get::<CommsArray>().is_some() {
                        interactable.kind = InteractableKind::CommsArray;
                        inventory_items.push("minimap_intel".to_string());
                    } else if target.get::<FabricationPlant>().is_some() {
                        interactable.kind = InteractableKind::FabricationPlant;
                        inventory_items.push("fab_plant_boost".to_string());
                    }
                }

                if let Ok(mut player_ent) = sim.world.get_entity_mut(player_ecs)
                    && let Some(mut inv) = player_ent.get_mut::<Inventory>()
                {
                    inv.items = inventory_items.clone();
                }

                if consumed {
                    sim.world
                        .resource_mut::<EntityRegistry>()
                        .unregister(target_id);
                    if let Ok(target) = sim.world.get_entity_mut(target_ecs) {
                        target.despawn();
                    }
                }

                send_to_entity(
                    connections,
                    entity_id,
                    &ServerMessage::InventoryUpdated {
                        entity_id,
                        items_json: serde_json::to_string(&inventory_items)
                            .unwrap_or_else(|_| "[]".to_string()),
                    },
                );
                send_to_entity(
                    connections,
                    entity_id,
                    &ServerMessage::InteractableUpdated { interactable },
                );
            }
            ClientMessage::AltFire {
                direction,
                client_latency_ms,
            } => {
                sim.world
                    .resource_mut::<InputQueue>()
                    .fires
                    .push(FireRequest {
                        entity_id,
                        direction,
                        client_latency_ms,
                    });
            }
            ClientMessage::DropItem { item, quantity } => {
                let Some(&ecs_entity) =
                    sim.world.resource::<EntityRegistry>().by_id.get(&entity_id)
                else {
                    return Ok(());
                };
                let drop_origin = {
                    let ent = sim.world.entity(ecs_entity);
                    (
                        ent.get::<Position>()
                            .map(|position| position.0)
                            .unwrap_or(Vec2::ZERO),
                        ent.get::<Zone>()
                            .map(|zone| zone.0)
                            .unwrap_or(ZoneId::Overworld),
                    )
                };
                let mut removed = 0u32;
                let mut updated_inventory: Vec<String> = Vec::new();
                if let Some(mut inv) = sim.world.entity_mut(ecs_entity).get_mut::<Inventory>() {
                    let mut removed_local = 0u32;
                    inv.items.retain(|owned| {
                        if removed_local < quantity && owned == &item {
                            removed_local += 1;
                            false
                        } else {
                            true
                        }
                    });
                    updated_inventory = inv.items.clone();
                    removed = removed_local;
                }
                for idx in 0..removed {
                    let id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
                    let world_offset = idx as f32 * 10.0;
                    let ecs = sim
                        .world
                        .spawn((
                            Item {
                                item_type: item.clone(),
                            },
                            Position(Vec2::new(
                                drop_origin.0.x + world_offset,
                                drop_origin.0.y + world_offset,
                            )),
                            Zone(drop_origin.1),
                            Collider { radius: 10.0 },
                        ))
                        .id();
                    sim.world.resource_mut::<EntityRegistry>().register(id, ecs);
                }
                if !updated_inventory.is_empty() || removed > 0 {
                    send_to_entity(
                        connections,
                        entity_id,
                        &ServerMessage::InventoryUpdated {
                            entity_id,
                            items_json: serde_json::to_string(&updated_inventory)
                                .unwrap_or_else(|_| "[]".to_string()),
                        },
                    );
                }
            }
            ClientMessage::UseItem { item } => {
                let Some(&ecs_entity) =
                    sim.world.resource::<EntityRegistry>().by_id.get(&entity_id)
                else {
                    return Ok(());
                };
                let mut used = false;
                if let Some(mut inv) = sim.world.entity_mut(ecs_entity).get_mut::<Inventory>()
                    && let Some(index) = inv.items.iter().position(|owned| owned == &item)
                    && item == "medkit"
                {
                    inv.items.remove(index);
                    used = true;
                }
                if used {
                    if let Some(mut health) = sim.world.entity_mut(ecs_entity).get_mut::<Health>() {
                        health.current = (health.current + 35.0).min(health.max);
                    }
                    let inv = sim
                        .world
                        .entity(ecs_entity)
                        .get::<Inventory>()
                        .map(|inv| inv.items.clone())
                        .unwrap_or_default();
                    send_to_entity(
                        connections,
                        entity_id,
                        &ServerMessage::InventoryUpdated {
                            entity_id,
                            items_json: serde_json::to_string(&inv)
                                .unwrap_or_else(|_| "[]".to_string()),
                        },
                    );
                }
            }
            ClientMessage::EquipItem { item, slot: _slot } => {
                let Some(&ecs_entity) =
                    sim.world.resource::<EntityRegistry>().by_id.get(&entity_id)
                else {
                    return Ok(());
                };
                let has_item = sim
                    .world
                    .entity(ecs_entity)
                    .get::<Inventory>()
                    .map(|inv| inv.items.iter().any(|owned| owned == &item))
                    .unwrap_or(false);
                if has_item {
                    sim.world
                        .resource_mut::<ProgressionQueue>()
                        .gains
                        .push(XpGainRequest {
                            entity_id,
                            branch: SkillBranch::Ballistics,
                            amount: 1,
                        });
                }
            }
            ClientMessage::RequestCraftingState => {
                send_to_entity(
                    connections,
                    entity_id,
                    &ServerMessage::CraftingCatalog {
                        items: lithos_world::content_catalog::item_definitions(),
                        recipes: lithos_world::content_catalog::recipe_definitions(),
                    },
                );
            }
            ClientMessage::RequestPowerState => {
                let zone = sim
                    .world
                    .resource::<EntityRegistry>()
                    .by_id
                    .get(&entity_id)
                    .copied()
                    .and_then(|ecs| sim.world.entity(ecs).get::<Zone>().map(|zone| zone.0))
                    .unwrap_or(ZoneId::Overworld);

                let mut generation_kw = 0.0_f32;
                let mut load_kw = 0.0_f32;
                let mut consumers_total = 0_u32;
                let mut consumers_powered = 0_u32;

                let mut gen_query = sim.world.query::<(&Zone, &PowerGenerator)>();
                for (generator_zone, generator) in gen_query.iter(&sim.world) {
                    if generator_zone.0 == zone && generator.fuel_remaining > 0.0 {
                        generation_kw += generator.output_kw;
                    }
                }

                let mut consumer_query = sim.world.query::<(&Zone, &PowerConsumer)>();
                for (consumer_zone, consumer) in consumer_query.iter(&sim.world) {
                    if consumer_zone.0 != zone {
                        continue;
                    }
                    consumers_total += 1;
                    load_kw += consumer.required_kw;
                    if consumer.is_powered {
                        consumers_powered += 1;
                    }
                }

                send_to_entity(
                    connections,
                    entity_id,
                    &ServerMessage::PowerState {
                        zone,
                        networks: vec![lithos_protocol::PowerNetworkSnapshot {
                            network_id: 1,
                            zone,
                            generation_kw,
                            load_kw,
                            consumers_powered,
                            consumers_total,
                        }],
                    },
                );
            }
            ClientMessage::StartHack { target_entity_id } => {
                let Some(&target_ecs) = sim
                    .world
                    .resource::<EntityRegistry>()
                    .by_id
                    .get(&target_entity_id)
                else {
                    return Ok(());
                };
                if let Some(mut hack) = sim.world.entity_mut(target_ecs).get_mut::<HackingTarget>()
                    && !hack.is_hacked
                {
                    hack.is_hacked = true;
                    sim.world
                        .resource_mut::<ProgressionQueue>()
                        .gains
                        .push(XpGainRequest {
                            entity_id,
                            branch: SkillBranch::Cybernetics,
                            amount: 12,
                        });
                }
            }
            ClientMessage::CancelHack => {}
            ClientMessage::RequestRaidTargets => {
                let requester_faction = connections.get(entity_id).and_then(|conn| conn.faction_id);
                let mut defenders: Vec<u64> = connections
                    .iter()
                    .filter_map(|conn| conn.faction_id)
                    .filter(|faction_id| Some(*faction_id) != requester_faction)
                    .collect();
                defenders.sort_unstable();
                defenders.dedup();
                send_to_entity(
                    connections,
                    entity_id,
                    &ServerMessage::RaidTargets {
                        defender_faction_ids: defenders,
                    },
                );
            }
            ClientMessage::InitiateRaid {
                defender_faction_id,
            } => {
                let Some(attacker_faction_id) =
                    connections.get(entity_id).and_then(|conn| conn.faction_id)
                else {
                    send_to_entity(
                        connections,
                        entity_id,
                        &ServerMessage::Disconnect {
                            reason: "cannot initiate raid without faction".to_string(),
                        },
                    );
                    return Ok(());
                };

                if attacker_faction_id == defender_faction_id {
                    return Ok(());
                }
                let defender_online = connections
                    .iter()
                    .any(|conn| conn.faction_id == Some(defender_faction_id));
                if !defender_online {
                    send_to_entity(
                        connections,
                        entity_id,
                        &ServerMessage::TradeFailed {
                            reason: "defender faction is offline".to_string(),
                        },
                    );
                    return Ok(());
                }

                let Some(&attacker_ecs) =
                    sim.world.resource::<EntityRegistry>().by_id.get(&entity_id)
                else {
                    return Ok(());
                };
                let mut has_breach_generator = false;
                if let Some(mut inv) = sim.world.entity_mut(attacker_ecs).get_mut::<Inventory>()
                    && let Some(idx) = inv.items.iter().position(|item| item == "breach_generator")
                {
                    inv.items.remove(idx);
                    has_breach_generator = true;
                    send_to_entity(
                        connections,
                        entity_id,
                        &ServerMessage::InventoryUpdated {
                            entity_id,
                            items_json: serde_json::to_string(&inv.items)
                                .unwrap_or_else(|_| "[]".to_string()),
                        },
                    );
                }
                if !has_breach_generator {
                    send_to_entity(
                        connections,
                        entity_id,
                        &ServerMessage::TradeFailed {
                            reason: "breach_generator required".to_string(),
                        },
                    );
                    return Ok(());
                }

                let tick = sim.current_tick();
                let dt = sim.world.resource::<SimConfig>().dt;
                let raid = RaidState {
                    attacker_faction_id,
                    defender_faction_id,
                    warning_ends_at_tick: tick + 120,
                    breach_ends_at_tick: tick + 420,
                    breach_active: false,
                };

                sim.world
                    .resource_mut::<RaidStateStore>()
                    .raids
                    .push(raid.clone());

                let warning = ServerMessage::RaidWarning {
                    raid: raid_snapshot(&raid, tick, dt),
                };
                send_to_faction(connections, defender_faction_id, &warning);
                send_to_faction(connections, attacker_faction_id, &warning);
            }
        },
        NetworkEvent::Disconnected { entity_id } => {
            unauth_connections.remove(&entity_id);
            let removed_conn = connections.remove(entity_id);

            let ecs_entity = sim
                .world
                .resource::<EntityRegistry>()
                .by_id
                .get(&entity_id)
                .copied();

            if let Some(ecs_entity) = ecs_entity {
                if let Some(conn) = &removed_conn {
                    let entity_ref = sim.world.entity(ecs_entity);
                    let pos = entity_ref
                        .get::<Position>()
                        .map(|p| p.0)
                        .unwrap_or(Vec2::ZERO);
                    let hp = entity_ref
                        .get::<Health>()
                        .map(|h| h.current)
                        .unwrap_or(100.0);
                    let inv = entity_ref
                        .get::<Inventory>()
                        .map(|i| {
                            serde_json::to_string(&i.items).unwrap_or_else(|_| "[]".to_string())
                        })
                        .unwrap_or_else(|| "[]".to_string());

                    let _ = sqlx::query(
                        "UPDATE players SET x = $1, y = $2, health = $3, inventory = $4, last_login = NOW() WHERE username = $5",
                    )
                    .bind(pos.x as f64)
                    .bind(pos.y as f64)
                    .bind(hp as f64)
                    .bind(inv)
                    .bind(&conn.username)
                    .execute(pool)
                    .await;
                }

                let player_id = sim.world.entity(ecs_entity).get::<Player>().map(|p| p.id);
                let mut registry = sim.world.resource_mut::<EntityRegistry>();
                if let Some(player_id) = player_id {
                    registry.player_entities.remove(&player_id);
                }
                registry.unregister(entity_id);
                let _ = sim.world.despawn(ecs_entity);
            }
        }
    }

    Ok(())
}

async fn flush_player_states(
    sim: &Simulation,
    connections: &ConnectionManager,
    pool: &sqlx::PgPool,
) {
    use futures::future::join_all;

    let mut futures = Vec::new();
    for conn in connections.iter() {
        if let Some(&ecs_entity) = sim
            .world
            .resource::<EntityRegistry>()
            .by_id
            .get(&conn.entity_id)
        {
            let entity_ref = sim.world.entity(ecs_entity);
            let pos = entity_ref
                .get::<Position>()
                .map(|p| p.0)
                .unwrap_or(Vec2::ZERO);
            let hp = entity_ref
                .get::<Health>()
                .map(|h| h.current)
                .unwrap_or(100.0);
            let inv = entity_ref
                .get::<Inventory>()
                .map(|i| serde_json::to_string(&i.items).unwrap_or_else(|_| "[]".to_string()))
                .unwrap_or_else(|| "[]".to_string());

            let username = conn.username.clone();
            let pool = pool.clone();
            futures.push(async move {
                let _ = sqlx::query(
                    "UPDATE players SET x = $1, y = $2, health = $3, inventory = $4 WHERE username = $5",
                )
                .bind(pos.x as f64)
                .bind(pos.y as f64)
                .bind(hp as f64)
                .bind(inv)
                .bind(&username)
                .execute(&pool)
                .await;
            });
        }
    }

    join_all(futures).await;
}

/// Build and send state snapshots to all connected clients.
fn broadcast_snapshots(sim: &mut Simulation, connections: &ConnectionManager) {
    if connections.count() == 0 {
        return;
    }

    let tick = sim.current_tick();
    let entity_map = sim.world.resource::<EntityRegistry>().by_entity.clone();

    use lithos_protocol::SnapshotEntityType;
    use lithos_world::components::{Item, Npc, Projectile, ResourceNode};

    let mut entities = Vec::new();
    let mut query = sim.world.query::<(
        bevy_ecs::entity::Entity,
        &Position,
        &Velocity,
        &Zone,
        Option<&Player>,
        Option<&Npc>,
        Option<&ResourceNode>,
        Option<&Item>,
        Option<&Projectile>,
    )>();

    for (ecs_entity, pos, vel, zone, player, npc, node, item, proj) in query.iter(&sim.world) {
        let Some(&eid) = entity_map.get(&ecs_entity) else {
            continue;
        };
        let entity_type = if player.is_some() {
            SnapshotEntityType::Player
        } else if let Some(npc) = npc {
            match npc.npc_type {
                NpcType::Rover => SnapshotEntityType::Rover,
                NpcType::Drone => SnapshotEntityType::Drone,
                NpcType::AssaultWalker => SnapshotEntityType::AssaultWalker,
                NpcType::SniperWalker => SnapshotEntityType::SniperWalker,
                NpcType::HeavyFlamethrower => SnapshotEntityType::HeavyFlamethrower,
                NpcType::CoreWarden => SnapshotEntityType::CoreWarden,
                NpcType::Trader => SnapshotEntityType::Trader,
            }
        } else if node.is_some() {
            SnapshotEntityType::ResourceNode
        } else if item.is_some() {
            SnapshotEntityType::Item
        } else if proj.is_some() {
            SnapshotEntityType::Projectile
        } else {
            SnapshotEntityType::Unknown
        };

        entities.push(EntitySnapshot {
            id: eid,
            position: pos.0,
            velocity: vel.0,
            zone: zone.0,
            entity_type,
        });
    }

    let last_seq_for_clients: HashMap<EntityId, u32> = {
        let map = &sim.world.resource::<LastProcessedSeq>().map;
        connections
            .iter()
            .map(|c| (c.entity_id, map.get(&c.entity_id).copied().unwrap_or(0)))
            .collect()
    };

    for conn in connections.iter() {
        let mut client_pos = Vec2::ZERO;
        let mut client_zone = ZoneId::Overworld;
        for entity in &entities {
            if entity.id == conn.entity_id {
                client_pos = entity.position;
                client_zone = entity.zone;
                break;
            }
        }

        let mut visible_entities = Vec::with_capacity(entities.len());
        for entity in &entities {
            if entity.zone != client_zone {
                continue;
            }
            let dist_sq = (entity.position - client_pos).length_squared();
            if dist_sq < 1500.0 * 1500.0 {
                visible_entities.push(*entity);
            }
        }

        let last_processed_seq = last_seq_for_clients
            .get(&conn.entity_id)
            .copied()
            .unwrap_or(0);
        encode_and_send(
            &conn.outbound_tx,
            &ServerMessage::StateSnapshot {
                tick,
                last_processed_seq,
                entities: visible_entities,
            },
        );
    }
}

fn send_zone_changes(sim: &mut Simulation, connections: &ConnectionManager) {
    let events = sim.world.resource::<ZoneChangeEvents>().events.clone();
    for event in events {
        send_to_entity(
            connections,
            event.entity_id,
            &ServerMessage::ZoneChanged {
                zone: event.new_zone,
            },
        );
    }
}

fn send_combat_events(sim: &mut Simulation, connections: &ConnectionManager) {
    let events = sim
        .world
        .resource::<lithos_world::resources::CombatEvents>();

    for event in &events.spawn_projectiles {
        broadcast_all(
            connections,
            &ServerMessage::SpawnProjectile {
                entity_id: event.entity_id,
                position: event.position,
                velocity: event.velocity,
            },
        );
    }

    for event in &events.health_changes {
        broadcast_all(
            connections,
            &ServerMessage::HealthChanged {
                entity_id: event.entity_id,
                health: event.health,
                max_health: event.max_health,
            },
        );
    }

    for event in &events.oxygen_changes {
        broadcast_all(
            connections,
            &ServerMessage::OxygenChanged {
                entity_id: event.entity_id,
                current: event.current,
                max: event.max,
            },
        );
    }

    for event in &events.deaths {
        broadcast_all(
            connections,
            &ServerMessage::PlayerDied {
                entity_id: event.entity_id,
            },
        );
    }

    for event in &events.inventory_updates {
        send_to_entity(
            connections,
            event.entity_id,
            &ServerMessage::InventoryUpdated {
                entity_id: event.entity_id,
                items_json: event.items_json.clone(),
            },
        );
    }

    for event in &events.credits_changes {
        send_to_faction(
            connections,
            event.faction_id,
            &ServerMessage::CreditsChanged {
                faction_id: event.faction_id,
                balance: event.balance,
            },
        );
    }

    for event in &events.progression_updates {
        send_to_entity(
            connections,
            event.entity_id,
            &ServerMessage::ProgressionUpdated {
                entity_id: event.entity_id,
                branches: event.branches.clone(),
            },
        );
    }

    for event in &events.ammo_changes {
        send_to_entity(
            connections,
            event.entity_id,
            &ServerMessage::AmmoChanged {
                entity_id: event.entity_id,
                ammo: event.ammo,
                max_ammo: event.max_ammo,
            },
        );
    }
}

fn send_mining_events(sim: &mut Simulation, connections: &ConnectionManager) {
    let (mining_events, depleted) = {
        let events = sim
            .world
            .resource::<lithos_world::resources::MiningEvents>();
        (events.events.clone(), events.depleted.clone())
    };

    for event in &mining_events {
        // Send inventory update.
        if let Some(&ecs_entity) = sim
            .world
            .resource::<EntityRegistry>()
            .by_id
            .get(&event.miner_entity_id)
            && let Some(inv) = sim.world.entity(ecs_entity).get::<Inventory>()
        {
            send_to_entity(
                connections,
                event.miner_entity_id,
                &ServerMessage::InventoryUpdated {
                    entity_id: event.miner_entity_id,
                    items_json: serde_json::to_string(&inv.items)
                        .unwrap_or_else(|_| "[]".to_string()),
                },
            );
        }
    }

    // Broadcast depleted nodes to all clients.
    for entity_id in &depleted {
        broadcast_all(
            connections,
            &ServerMessage::ResourceDepleted {
                entity_id: *entity_id,
            },
        );
    }

    // Despawn depleted resource nodes.
    for entity_id in depleted {
        if let Some(&ecs_entity) = sim.world.resource::<EntityRegistry>().by_id.get(&entity_id) {
            sim.world
                .resource_mut::<EntityRegistry>()
                .unregister(entity_id);
            sim.world.despawn(ecs_entity);
        }
    }
}

fn send_trade_events(sim: &mut Simulation, connections: &ConnectionManager) {
    let (events, failures) = {
        let trade = sim.world.resource::<lithos_world::resources::TradeEvents>();
        (trade.events.clone(), trade.failures.clone())
    };

    for event in &events {
        // Send inventory update.
        if let Some(&ecs_entity) = sim
            .world
            .resource::<EntityRegistry>()
            .by_id
            .get(&event.entity_id)
            && let Some(inv) = sim.world.entity(ecs_entity).get::<Inventory>()
        {
            send_to_entity(
                connections,
                event.entity_id,
                &ServerMessage::InventoryUpdated {
                    entity_id: event.entity_id,
                    items_json: serde_json::to_string(&inv.items)
                        .unwrap_or_else(|_| "[]".to_string()),
                },
            );
        }

        // Send faction credits update.
        if let Some(&ecs_entity) = sim
            .world
            .resource::<EntityRegistry>()
            .by_id
            .get(&event.entity_id)
            && let Some(player) = sim.world.entity(ecs_entity).get::<Player>()
            && let Some(faction_id) = player.faction_id
        {
            let balance = sim
                .world
                .resource::<FactionVaults>()
                .balances
                .get(&faction_id)
                .copied()
                .unwrap_or(0);
            send_to_entity(
                connections,
                event.entity_id,
                &ServerMessage::CreditsChanged {
                    faction_id,
                    balance,
                },
            );
        }
    }

    for (entity_id, reason) in failures {
        send_to_entity(
            connections,
            entity_id,
            &ServerMessage::TradeFailed {
                reason: reason.clone(),
            },
        );
    }
}

fn send_chat_events(sim: &mut Simulation, connections: &ConnectionManager) {
    let messages = {
        let mut bus = sim.world.resource_mut::<ChatEvents>();
        std::mem::take(&mut bus.messages)
    };

    for msg in messages {
        let outbound = ServerMessage::ChatMessage {
            from_entity_id: msg.from_entity_id,
            channel: msg.channel,
            text: msg.text,
            sent_at_unix_ms: msg.sent_at_unix_ms,
        };

        match msg.channel {
            ChatChannel::Global => broadcast_all(connections, &outbound),
            ChatChannel::Faction => {
                if let Some(faction_id) = msg.faction_id {
                    send_to_faction(connections, faction_id, &outbound);
                }
            }
        }
    }
}

fn send_dynamic_events(sim: &mut Simulation, connections: &ConnectionManager) {
    let started = sim
        .world
        .resource::<lithos_world::resources::DynamicEventBus>()
        .started
        .clone();
    let ended = sim
        .world
        .resource::<lithos_world::resources::DynamicEventBus>()
        .ended_event_ids
        .clone();

    let dt = sim.world.resource::<SimConfig>().dt;
    for event in started {
        let started_at = now_unix_ms();
        let ttl_ticks = event.expires_at_tick.saturating_sub(event.started_at_tick);
        let expires_at = started_at + (ttl_ticks as f32 * dt * 1000.0) as u64;
        broadcast_all(
            connections,
            &ServerMessage::DynamicEventStarted {
                event: DynamicEventSnapshot {
                    event_id: event.event_id,
                    kind: event.kind,
                    started_at_unix_ms: started_at,
                    expires_at_unix_ms: expires_at,
                    description: event.description,
                },
            },
        );
    }

    for event_id in ended {
        broadcast_all(connections, &ServerMessage::DynamicEventEnded { event_id });
    }
}

fn send_raid_events(sim: &mut Simulation, connections: &ConnectionManager) {
    let started = sim
        .world
        .resource::<lithos_world::resources::RaidEventBus>()
        .started
        .clone();
    let ended = sim
        .world
        .resource::<lithos_world::resources::RaidEventBus>()
        .ended
        .clone();
    let tick = sim.current_tick();
    let dt = sim.world.resource::<SimConfig>().dt;

    for raid in started {
        let snapshot = raid_snapshot(&raid, tick, dt);
        let msg = ServerMessage::RaidStarted { raid: snapshot };
        send_to_faction(connections, raid.attacker_faction_id, &msg);
        send_to_faction(connections, raid.defender_faction_id, &msg);
    }

    for (raid, attacker_won) in ended {
        let snapshot = raid_snapshot(&raid, tick, dt);
        let msg = ServerMessage::RaidEnded {
            raid: snapshot,
            attacker_won,
        };
        send_to_faction(connections, raid.attacker_faction_id, &msg);
        send_to_faction(connections, raid.defender_faction_id, &msg);
    }
}

/// Load chunks around all connected players and send `WorldMapChunk` messages
/// for any chunks the player hasn't seen yet.
async fn load_and_send_chunks(
    sim: &mut Simulation,
    connections: &ConnectionManager,
    radius: i32,
    player_loaded_chunks: &mut HashMap<EntityId, std::collections::HashSet<ChunkCoord>>,
) {
    use lithos_protocol::TileData;

    // Collect player positions.
    let player_positions: Vec<(EntityId, Vec2)> = {
        let registry = sim.world.resource::<EntityRegistry>();
        let mut positions = Vec::new();
        for conn in connections.iter() {
            if let Some(&ecs_entity) = registry.by_id.get(&conn.entity_id)
                && let Some(pos) = sim.world.entity(ecs_entity).get::<Position>()
            {
                positions.push((conn.entity_id, pos.0));
            }
        }
        positions
    };

    // Gather loader positions for chunk unloading.
    let loader_positions: Vec<Vec2> = player_positions.iter().map(|(_, p)| *p).collect();

    // Unload chunks far from all players.
    {
        let mut tilemap = sim.world.resource_mut::<TileMap>();
        tilemap.unload_distant_chunks(&loader_positions, radius + 2);
    }

    // For each player, ensure nearby chunks are loaded and send new ones.
    for (entity_id, pos) in player_positions {
        let center = ChunkCoord::from_world_pos(pos);
        let loaded = player_loaded_chunks.entry(entity_id).or_default();

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let coord = ChunkCoord {
                    x: center.x + dx,
                    y: center.y + dy,
                };

                if loaded.contains(&coord) {
                    continue;
                }

                // Generate/load the chunk.
                let chunk = {
                    let mut tilemap = sim.world.resource_mut::<TileMap>();
                    tilemap.ensure_chunk(coord);
                    tilemap.get_chunk(coord).cloned()
                };

                if let Some(chunk) = chunk {
                    // Convert to protocol TileData.
                    let tiles: Vec<TileData> = chunk
                        .tiles
                        .iter()
                        .map(|t| TileData {
                            terrain: match t.terrain {
                                lithos_world::tilemap::TerrainType::Empty => {
                                    lithos_protocol::TerrainType::Empty
                                }
                                lithos_world::tilemap::TerrainType::Rock => {
                                    lithos_protocol::TerrainType::Rock
                                }
                                lithos_world::tilemap::TerrainType::DeepRavine => {
                                    lithos_protocol::TerrainType::DeepRavine
                                }
                                lithos_world::tilemap::TerrainType::AsteroidField => {
                                    lithos_protocol::TerrainType::AsteroidField
                                }
                                lithos_world::tilemap::TerrainType::AutomataSpire => {
                                    lithos_protocol::TerrainType::AutomataSpire
                                }
                            },
                            ceiling: match t.ceiling {
                                lithos_world::tilemap::CeilingType::Open => {
                                    lithos_protocol::CeilingType::Open
                                }
                                lithos_world::tilemap::CeilingType::Enclosed => {
                                    lithos_protocol::CeilingType::Enclosed
                                }
                            },
                            height: t.height,
                        })
                        .collect();

                    let msg = ServerMessage::WorldMapChunk {
                        chunk_x: coord.x,
                        chunk_y: coord.y,
                        tiles,
                    };
                    send_to_entity(connections, entity_id, &msg);
                    loaded.insert(coord);
                }
            }
        }
    }
}
