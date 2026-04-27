//! The main game loop — ties together networking, ECS simulation, and broadcasting.

use anyhow::{Context, Result};
use rand::Rng;
use serde::Serialize;
use sqlx::Row;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use lithos_protocol::{
    ChatChannel, ClientMessage, DynamicEventSnapshot, EntityId, EntitySnapshot, RaidStateSnapshot,
    ServerMessage, SkillBranch, Vec2, ZoneId, codec,
};
use lithos_world::components::{
    BaseTile, Collider, Health, Inventory, Npc, NpcState, NpcType, Oxygen, Player, Position,
    PositionHistory, PowerConsumer, PowerGenerator, Progression, ResourceNode, ResourceType,
    TileType, Velocity, Weapon, Zone,
};
use lithos_world::resources::{
    ChatEvent, ChatEvents, EntityRegistry, FactionVaults, FireRequest, InputQueue,
    LastProcessedSeq, MoveInput, ProgressionQueue, RaidState, RaidStateStore, RespawnRequest,
    SimConfig, TraderMarket, XpGainRequest, ZoneChangeEvents, ZoneTransferRequest,
};
use lithos_world::simulation::Simulation;
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

fn encode_and_send(outbound: &mpsc::UnboundedSender<Vec<u8>>, msg: &ServerMessage) {
    if let Ok(bytes) = codec::encode(msg) {
        let _ = outbound.send(bytes);
    }
}

fn send_to_entity(connections: &ConnectionManager, entity_id: EntityId, msg: &ServerMessage) {
    if let Some(conn) = connections.get(entity_id) {
        encode_and_send(&conn.outbound_tx, msg);
    }
}

fn broadcast_all(connections: &ConnectionManager, msg: &ServerMessage) {
    if let Ok(bytes) = codec::encode(msg) {
        for conn in connections.iter() {
            let _ = conn.outbound_tx.send(bytes.clone());
        }
    }
}

fn send_to_faction(connections: &ConnectionManager, faction_id: u64, msg: &ServerMessage) {
    if let Ok(bytes) = codec::encode(msg) {
        for conn in connections.iter() {
            if conn.faction_id == Some(faction_id) {
                let _ = conn.outbound_tx.send(bytes.clone());
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
    let mut rng = rand::thread_rng();
    let generator = WorldGenerator::new(world_seed);

    // Hostiles
    for _ in 0..100 {
        let pos = Vec2::new(
            rng.gen_range(-4000.0..4000.0),
            rng.gen_range(-4000.0..4000.0),
        );
        let biome = generator.get_biome(pos);
        if biome == Biome::OuterRim {
            continue;
        }

        let npc_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
        let health = if biome == Biome::Core { 300.0 } else { 100.0 };
        let ecs_ent = sim
            .world
            .spawn((
                Position(pos),
                Velocity(Vec2::ZERO),
                Zone(ZoneId::Overworld),
                Npc {
                    npc_type: NpcType::Hostile,
                    state: NpcState::Patrol,
                    target: None,
                    spawn_pos: pos,
                },
                Health {
                    current: health,
                    max: health,
                },
                Weapon {
                    damage: if biome == Biome::Core { 40.0 } else { 15.0 },
                    projectile_speed: 400.0,
                    cooldown_seconds: 1.0,
                    last_fired_time: 0.0,
                },
                Collider { radius: 14.0 },
                Inventory {
                    items: vec!["scrap".to_string(), "circuit".to_string()],
                },
            ))
            .id();
        sim.world
            .resource_mut::<EntityRegistry>()
            .register(npc_id, ecs_ent);
    }

    // Traders
    for _ in 0..20 {
        let pos = Vec2::new(
            rng.gen_range(-4000.0..4000.0),
            rng.gen_range(-4000.0..4000.0),
        );
        let biome = generator.get_biome(pos);
        if biome == Biome::Core {
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

    // Resource nodes
    for _ in 0..200 {
        let pos = Vec2::new(
            rng.gen_range(-4000.0..4000.0),
            rng.gen_range(-4000.0..4000.0),
        );
        let biome = generator.get_biome(pos);
        let resource_type = match biome {
            Biome::OuterRim => ResourceType::Iron,
            Biome::MidZone => {
                if rng.gen_bool(0.3) {
                    ResourceType::Titanium
                } else {
                    ResourceType::Iron
                }
            }
            Biome::Core => {
                if rng.gen_bool(0.4) {
                    ResourceType::Lithos
                } else {
                    ResourceType::Titanium
                }
            }
        };

        let node_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
        let ecs_ent = sim
            .world
            .spawn((
                Position(pos),
                Zone(ZoneId::Overworld),
                Collider { radius: 20.0 },
                ResourceNode {
                    resource_type,
                    yield_amount: rng.gen_range(5..15),
                },
            ))
            .id();
        sim.world
            .resource_mut::<EntityRegistry>()
            .register(node_id, ecs_ent);
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
    let mut sim = Simulation::new();
    let mut connections = ConnectionManager::new();
    let mut unauth_connections = HashMap::new();

    seed_world(&mut sim, config.world_seed);
    tracing::info!(tick_rate = config.tick_rate, "game loop starting");

    let mut flush_counter = 0_u64;
    let mut heartbeat_counter = 0_u64;

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
        send_chat_events(&mut sim, &connections);
        send_dynamic_events(&mut sim, &connections);
        send_raid_events(&mut sim, &connections);
        broadcast_snapshots(&mut sim, &connections);

        flush_counter += 1;
        if flush_counter.is_multiple_of(60) {
            flush_player_states(&sim, &connections, &pool).await;
        }

        heartbeat_counter += 1;
        if heartbeat_counter.is_multiple_of(100) {
            report_heartbeat(&config, connections.count()).await;
        }

        let elapsed = tick_start.elapsed();
        if elapsed < tick_duration {
            tokio::time::sleep(tick_duration - elapsed).await;
        } else {
            tracing::warn!(
                elapsed_ms = elapsed.as_millis(),
                budget_ms = tick_duration.as_millis(),
                "tick overran budget"
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_event(
    event: NetworkEvent,
    sim: &mut Simulation,
    connections: &mut ConnectionManager,
    unauth_connections: &mut HashMap<EntityId, mpsc::UnboundedSender<Vec<u8>>>,
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
                        },
                        Collider { radius: 14.0 },
                        Inventory { items: vec![] },
                        Oxygen {
                            current: 100.0,
                            max: 100.0,
                        },
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
                sim.world
                    .resource_mut::<InputQueue>()
                    .zone_transfers
                    .push(ZoneTransferRequest { entity_id, target });
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
                        TileType::Door | TileType::Workbench => {
                            entity.insert(PowerConsumer {
                                required_kw: 10.0,
                                is_powered: false,
                            });
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

            let _ = sqlx::query(
                "UPDATE players SET x = $1, y = $2, health = $3, inventory = $4 WHERE username = $5",
            )
            .bind(pos.x as f64)
            .bind(pos.y as f64)
            .bind(hp as f64)
            .bind(inv)
            .bind(&conn.username)
            .execute(pool)
            .await;
        }
    }
}

/// Build and send state snapshots to all connected clients.
fn broadcast_snapshots(sim: &mut Simulation, connections: &ConnectionManager) {
    if connections.count() == 0 {
        return;
    }

    let tick = sim.current_tick();
    let last_seq_map = sim.world.resource::<LastProcessedSeq>().map.clone();
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
                NpcType::Hostile => SnapshotEntityType::Hostile,
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
                visible_entities.push(entity.clone());
            }
        }

        encode_and_send(
            &conn.outbound_tx,
            &ServerMessage::StateSnapshot {
                tick,
                last_processed_seq: last_seq_map.get(&conn.entity_id).copied().unwrap_or(0),
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
