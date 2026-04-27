//! The main game loop — ties together networking, ECS simulation, and broadcasting.

use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use lithos_protocol::{
    ClientMessage, EntityId, EntitySnapshot, PlayerId, ServerMessage, Vec2, ZoneId, codec,
};
use lithos_world::components::{Player, Position, Velocity, Zone};
use lithos_world::resources::{EntityRegistry, InputQueue, LastProcessedSeq, MoveInput, ZoneChangeEvents, ZoneTransferRequest, FireRequest, RespawnRequest};
use lithos_world::simulation::Simulation;

use crate::connection::ConnectionManager;
use crate::network::{self, NetworkEvent};
use crate::ServerConfig;

/// Run the game server.
pub async fn run(config: ServerConfig) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&config.listen_addr).await?;
    tracing::info!(addr = %config.listen_addr, "WebSocket listener ready");

    // Channel for network events → game loop.
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<NetworkEvent>();

    // Spawn the TCP accept loop.
    let accept_event_tx = event_tx.clone();
    tokio::spawn(async move {
        let mut next_id: u64 = 1;
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let entity_id = EntityId(next_id);
                    next_id += 1;
                    let tx = accept_event_tx.clone();
                    tokio::spawn(network::handle_connection(stream, entity_id, tx));
                }
                Err(e) => {
                    tracing::error!("accept error: {e}");
                }
            }
        }
    });

    // Initialize the game simulation.
    let tick_duration = Duration::from_secs_f64(1.0 / config.tick_rate as f64);
    let mut sim = Simulation::new();
    let mut connections = ConnectionManager::new();

    // Spawn some initial entities
    use lithos_world::world_gen::{WorldGenerator, Biome};
    use lithos_world::components::{Npc, NpcState, NpcType, Health, Weapon, Collider, Inventory, ResourceNode, ResourceType};
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let generator = WorldGenerator::new(config.world_seed);
    
    // Spawn Hostiles
    for _ in 0..100 {
        let x = rng.gen_range(-4000.0..4000.0);
        let y = rng.gen_range(-4000.0..4000.0);
        let pos = Vec2::new(x, y);
        let biome = generator.get_biome(pos);
        
        if biome != Biome::OuterRim {
            let npc_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
            let health = if biome == Biome::Core { 300.0 } else { 100.0 };
            
            let ecs_ent = sim.world.spawn((
                Position(pos),
                Velocity(Vec2::ZERO),
                Zone(ZoneId::Overworld),
                Npc {
                    npc_type: NpcType::Hostile,
                    state: NpcState::Patrol,
                    target: None,
                    spawn_pos: pos,
                },
                Health { current: health, max: health },
                Weapon {
                    damage: if biome == Biome::Core { 40.0 } else { 15.0 },
                    projectile_speed: 400.0,
                    cooldown_seconds: 1.0,
                    last_fired_time: 0.0,
                },
                Collider { radius: 14.0 },
                Inventory { items: vec!["scrap".to_string(), "circuit".to_string()] },
            )).id();
            sim.world.resource_mut::<EntityRegistry>().register(npc_id, ecs_ent);
        }
    }
    
    // Spawn Traders (Outer Rim & Mid-Zone only)
    for _ in 0..20 {
        let x = rng.gen_range(-4000.0..4000.0);
        let y = rng.gen_range(-4000.0..4000.0);
        let pos = Vec2::new(x, y);
        let biome = generator.get_biome(pos);
        
        if biome != Biome::Core {
            let npc_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
            let ecs_ent = sim.world.spawn((
                Position(pos),
                Velocity(Vec2::ZERO),
                Zone(ZoneId::Overworld),
                Npc {
                    npc_type: NpcType::Trader,
                    state: NpcState::Patrol,
                    target: None,
                    spawn_pos: pos,
                },
                Health { current: 500.0, max: 500.0 }, // Tough Traders
                Collider { radius: 14.0 },
                Inventory { items: vec!["medkit".to_string(), "battery".to_string()] },
            )).id();
            sim.world.resource_mut::<EntityRegistry>().register(npc_id, ecs_ent);
        }
    }
    
    // Spawn Resource Nodes (Radial Scaling)
    for _ in 0..200 {
        let x = rng.gen_range(-4000.0..4000.0);
        let y = rng.gen_range(-4000.0..4000.0);
        let pos = Vec2::new(x, y);
        let biome = generator.get_biome(pos);
        
        let r_type = match biome {
            Biome::OuterRim => ResourceType::Iron,
            Biome::MidZone => if rng.gen_bool(0.3) { ResourceType::Titanium } else { ResourceType::Iron },
            Biome::Core => if rng.gen_bool(0.4) { ResourceType::Lithos } else { ResourceType::Titanium },
        };
        
        let node_id = sim.world.resource_mut::<EntityRegistry>().next_entity_id();
        let ecs_ent = sim.world.spawn((
            Position(pos),
            Zone(ZoneId::Overworld),
            Collider { radius: 20.0 }, // Larger hit box
            ResourceNode {
                resource_type: r_type,
                yield_amount: rng.gen_range(5..15),
            },
        )).id();
        sim.world.resource_mut::<EntityRegistry>().register(node_id, ecs_ent);
    }

    tracing::info!(tick_rate = config.tick_rate, "game loop starting");

    // ── Main game loop ───────────────────────────────────────────────
    loop {
        let tick_start = Instant::now();

        // 1. Drain all network events.
        while let Ok(event) = event_rx.try_recv() {
            handle_event(event, &mut sim, &mut connections, config.world_seed);
        }

        // 2. Run one simulation tick.
        sim.tick();

        // 2.5. Send ZoneChanged messages for any zone transfers this tick.
        send_zone_changes(&mut sim, &connections);
        
        // 2.6. Send combat-related events.
        send_combat_events(&mut sim, &connections);

        // 3. Broadcast state snapshots to all connected clients.
        broadcast_snapshots(&mut sim, &connections);

        // 4. Sleep until the next tick.
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

/// Process a single network event.
fn handle_event(
    event: NetworkEvent,
    sim: &mut Simulation,
    connections: &mut ConnectionManager,
    world_seed: u32,
) {
    match event {
        NetworkEvent::Connected { entity_id, outbound_tx } => {
            let player_id = PlayerId::new();

            // Spawn a player entity in the ECS world.
            let ecs_entity = sim.world.spawn((
                Position(Vec2::ZERO),
                Velocity(Vec2::ZERO),
                Player { id: player_id },
                Zone(ZoneId::Overworld),
                lithos_world::components::Health { current: 100.0, max: 100.0 },
                lithos_world::components::Weapon { damage: 20.0, projectile_speed: 600.0, cooldown_seconds: 0.5, last_fired_time: 0.0 },
                lithos_world::components::Collider { radius: 14.0 },
                lithos_world::components::Inventory { items: vec![] },
            )).id();

            // Register in the entity registry.
            let mut registry = sim.world.resource_mut::<EntityRegistry>();
            registry.register(entity_id, ecs_entity);
            registry.player_entities.insert(player_id, entity_id);

            // Add to connection manager.
            connections.add(player_id, entity_id, outbound_tx.clone());

            // Send JoinAck to the client.
            let ack = ServerMessage::JoinAck {
                player_id,
                entity_id,
                zone: ZoneId::Overworld,
                world_seed,
            };
            if let Ok(bytes) = codec::encode(&ack) {
                let _ = outbound_tx.send(bytes);
            }
        }

        NetworkEvent::Message { entity_id, message } => {
            match message {
                ClientMessage::Join { .. } => {
                    // Join is handled on connect; ignore duplicate Joins.
                }
                ClientMessage::Move { direction, seq } => {
                    sim.world.resource_mut::<InputQueue>().moves.push(MoveInput {
                        entity_id,
                        direction,
                        seq,
                    });
                }
                ClientMessage::ZoneTransfer { target } => {
                    sim.world.resource_mut::<InputQueue>().zone_transfers.push(
                        ZoneTransferRequest { entity_id, target },
                    );
                }
                ClientMessage::Fire { direction } => {
                    sim.world.resource_mut::<InputQueue>().fires.push(
                        FireRequest { entity_id, direction },
                    );
                }
                ClientMessage::Respawn => {
                    sim.world.resource_mut::<InputQueue>().respawns.push(
                        RespawnRequest { entity_id },
                    );
                }
                ClientMessage::Ping { timestamp } => {
                    // Respond with Pong immediately.
                    let pong = ServerMessage::Pong {
                        client_timestamp: timestamp,
                        server_timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    };
                    if let Ok(bytes) = codec::encode(&pong) {
                        for conn in connections.iter() {
                            if conn.entity_id == entity_id {
                                let _ = conn.outbound_tx.send(bytes.clone());
                                break;
                            }
                        }
                    }
                }
            }
        }

        NetworkEvent::Disconnected { entity_id } => {
            // Remove from connection manager.
            connections.remove(entity_id);

            // Look up the ECS entity and player ID.
            let ecs_entity = sim.world.resource::<EntityRegistry>()
                .by_id.get(&entity_id).copied();

            if let Some(ecs_entity) = ecs_entity {
                // Read player_id before taking a mutable borrow on registry.
                let player_id = sim.world.entity(ecs_entity)
                    .get::<Player>()
                    .map(|p| p.id);

                let mut registry = sim.world.resource_mut::<EntityRegistry>();
                if let Some(pid) = player_id {
                    registry.player_entities.remove(&pid);
                }
                registry.unregister(entity_id);
                sim.world.despawn(ecs_entity);
            }
        }
    }
}

/// Build and send state snapshots to all connected clients.
fn broadcast_snapshots(sim: &mut Simulation, connections: &ConnectionManager) {
    if connections.count() == 0 {
        return;
    }

    let tick = sim.current_tick();

    // Clone the data we need from resources to avoid borrow conflicts with World::query.
    let last_seq_map = sim.world.resource::<LastProcessedSeq>().map.clone();
    let entity_map = sim.world.resource::<EntityRegistry>().by_entity.clone();

    // Build the entity snapshot list.
    use lithos_world::components::{Npc, ResourceNode, Item, Projectile, NpcType};
    use lithos_protocol::SnapshotEntityType;
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
        if let Some(&eid) = entity_map.get(&ecs_entity) {
            let entity_type = if player.is_some() {
                SnapshotEntityType::Player
            } else if let Some(n) = npc {
                match n.npc_type {
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
    }

    // Send a personalized snapshot to each client (with their last_processed_seq and culled entities).
    for conn in connections.iter() {
        let mut client_pos = Vec2::ZERO;
        let mut client_zone = ZoneId::Overworld;
        
        // Find this client's position and zone to filter interest
        for e in &entities {
            if e.id == conn.entity_id {
                client_pos = e.position;
                client_zone = e.zone;
                break;
            }
        }

        // Filter entities: must be in same zone, and within interest radius (1500 units)
        let mut visible_entities = Vec::with_capacity(entities.len());
        for e in &entities {
            if e.zone == client_zone {
                let dist_sq = (e.position - client_pos).length_squared();
                if dist_sq < 1500.0 * 1500.0 {
                    visible_entities.push(e.clone());
                }
            }
        }

        let snapshot = ServerMessage::StateSnapshot {
            tick,
            last_processed_seq: last_seq_map.get(&conn.entity_id).copied().unwrap_or(0),
            entities: visible_entities,
        };
        if let Ok(bytes) = codec::encode(&snapshot) {
            let _ = conn.outbound_tx.send(bytes);
        }
    }
}

/// Send ZoneChanged messages for any zone transfers that occurred this tick.
fn send_zone_changes(sim: &mut Simulation, connections: &ConnectionManager) {
    let events = sim.world.resource::<ZoneChangeEvents>().events.clone();
    for event in events {
        let msg = ServerMessage::ZoneChanged { zone: event.new_zone };
        if let Ok(bytes) = codec::encode(&msg) {
            for conn in connections.iter() {
                if conn.entity_id == event.entity_id {
                    let _ = conn.outbound_tx.send(bytes);
                    break;
                }
            }
        }
    }
}
/// Send combat-related events to clients.
fn send_combat_events(sim: &mut Simulation, connections: &ConnectionManager) {
    let combat_events = sim.world.resource::<lithos_world::resources::CombatEvents>();

    // Spawn Projectiles
    for event in &combat_events.spawn_projectiles {
        let msg = ServerMessage::SpawnProjectile {
            entity_id: event.entity_id,
            position: event.position,
            velocity: event.velocity,
        };
        if let Ok(bytes) = codec::encode(&msg) {
            for conn in connections.iter() {
                let _ = conn.outbound_tx.send(bytes.clone());
            }
        }
    }

    // Health Changes
    for event in &combat_events.health_changes {
        let msg = ServerMessage::HealthChanged {
            entity_id: event.entity_id,
            health: event.health,
            max_health: event.max_health,
        };
        if let Ok(bytes) = codec::encode(&msg) {
            for conn in connections.iter() {
                let _ = conn.outbound_tx.send(bytes.clone());
            }
        }
    }

    // Deaths
    for event in &combat_events.deaths {
        let msg = ServerMessage::PlayerDied {
            entity_id: event.entity_id,
        };
        if let Ok(bytes) = codec::encode(&msg) {
            for conn in connections.iter() {
                let _ = conn.outbound_tx.send(bytes.clone());
            }
        }
    }

    // Inventory Updates
    for event in &combat_events.inventory_updates {
        let msg = ServerMessage::InventoryUpdated {
            entity_id: event.entity_id,
            items_json: event.items_json.clone(),
        };
        if let Ok(bytes) = codec::encode(&msg) {
            for conn in connections.iter() {
                let _ = conn.outbound_tx.send(bytes.clone());
            }
        }
    }
}
