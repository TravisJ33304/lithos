//! ECS systems — game logic that runs each tick.

use bevy_ecs::prelude::*;

use crate::components::{
    Collider, Dead, Health, Inventory, Item, LastLoadoutTick, Npc, NpcState, NpcType, Player,
    Position, PositionHistory, Progression, Projectile, ResourceNode, ResourceType, Velocity,
    Weapon, Zone,
};
use crate::resources::{
    ActiveDynamicEvents, CombatEvents, DynamicEventBus, EntityRegistry, FactionVaults,
    HealthChangedEvent, InputQueue, InventoryUpdatedEvent, LastProcessedSeq, MineQueue,
    MiningEvent, MiningEvents, PlayerDiedEvent, ProgressionQueue, ProgressionUpdatedEvent,
    RaidEventBus, RaidStateStore, SimConfig, SpawnProjectileEvent, TickCounter, TraderMarket,
    XpGainRequest, ZoneChangeEvent, ZoneChangeEvents,
};

/// Advance the tick counter.
pub fn tick_counter_system(mut counter: ResMut<TickCounter>) {
    counter.tick += 1;
}

/// Drain the input queue and apply movement inputs to entities.
pub fn process_inputs_system(
    mut input_queue: ResMut<InputQueue>,
    mut last_seq: ResMut<LastProcessedSeq>,
    registry: Res<EntityRegistry>,
    config: Res<SimConfig>,
    mut query: Query<&mut Velocity, Without<Dead>>,
) {
    for input in input_queue.moves.drain(..) {
        if let Some(&ecs_entity) = registry.by_id.get(&input.entity_id)
            && let Ok(mut vel) = query.get_mut(ecs_entity)
        {
            // Normalize direction and apply max speed.
            let dir = input.direction.normalize();
            vel.0 = dir * config.max_speed;
            last_seq.map.insert(input.entity_id, input.seq);
        }
    }
}

/// Capture per-tick position history for lag compensation rewind.
pub fn position_history_system(
    tick: Res<TickCounter>,
    config: Res<SimConfig>,
    mut query: Query<(&Position, &mut PositionHistory), Without<Dead>>,
) {
    for (pos, mut history) in query.iter_mut() {
        history.samples.push_back((tick.tick, pos.0));
        while history.samples.len() as u64 > config.lag_comp_history_ticks {
            let _ = history.samples.pop_front();
        }
    }
}

/// Apply velocity to position (Euler integration).
pub fn movement_system(
    config: Res<SimConfig>,
    mut query: Query<(&mut Position, &Velocity), Without<Dead>>,
) {
    for (mut pos, vel) in query.iter_mut() {
        pos.0 += vel.0 * config.dt;
    }
}

/// Clamp positions to world bounds.
pub fn bounds_system(config: Res<SimConfig>, mut query: Query<&mut Position>) {
    let half = config.world_half_size;
    for mut pos in query.iter_mut() {
        pos.0.x = pos.0.x.clamp(-half, half);
        pos.0.y = pos.0.y.clamp(-half, half);
    }
}

/// Stop entities that received no input this tick (friction/deceleration).
/// For now, velocity persists until the client sends a zero-direction input.
/// This is a simple approach — proper deceleration can be added later.
pub fn friction_system(mut query: Query<&mut Velocity>) {
    // Currently a no-op: velocity is set directly from input each tick.
    // Players must send Move { direction: Vec2::ZERO } to stop.
    let _ = &mut query;
}

/// Process zone transfer requests.
pub fn zone_transfer_system(
    mut input_queue: ResMut<InputQueue>,
    registry: Res<EntityRegistry>,
    mut zone_events: ResMut<ZoneChangeEvents>,
    mut query: Query<&mut Zone, Without<Dead>>,
) {
    // Clear events from last tick.
    zone_events.events.clear();

    for req in input_queue.zone_transfers.drain(..) {
        if let Some(&ecs_entity) = registry.by_id.get(&req.entity_id)
            && let Ok(mut zone) = query.get_mut(ecs_entity)
        {
            tracing::info!(
                entity_id = req.entity_id.0,
                target = ?req.target,
                "zone transfer"
            );
            zone.0 = req.target;
            zone_events.events.push(ZoneChangeEvent {
                entity_id: req.entity_id,
                new_zone: req.target,
            });
        }
    }
}

/// Process weapon firing requests.
pub fn combat_system(
    mut commands: Commands,
    mut input_queue: ResMut<InputQueue>,
    mut registry: ResMut<EntityRegistry>,
    mut combat_events: ResMut<CombatEvents>,
    time: Res<TickCounter>,
    config: Res<SimConfig>,
    mut query: Query<(&Position, &mut Weapon, &Zone), Without<Dead>>,
) {
    let current_time = time.tick as f64 * config.dt as f64;

    // Clear old events
    combat_events.spawn_projectiles.clear();

    for req in input_queue.fires.drain(..) {
        if let Some(&ecs_entity) = registry.by_id.get(&req.entity_id)
            && let Ok((pos, mut weapon, zone)) = query.get_mut(ecs_entity)
            && current_time >= weapon.last_fired_time + weapon.cooldown_seconds as f64
        {
            weapon.last_fired_time = current_time;

            let dir = req.direction.normalize();
            if dir.x == 0.0 && dir.y == 0.0 {
                continue;
            }

            let proj_vel = dir * weapon.projectile_speed;
            // Spawn projectile slightly ahead
            let proj_pos = pos.0 + dir * 20.0;

            let new_id = registry.next_entity_id();
            let rewind_ticks = ((req.client_latency_ms as f32 / 1000.0) / config.dt)
                .round()
                .clamp(0.0, config.lag_comp_history_ticks as f32)
                as u32;
            let new_ecs_entity = commands
                .spawn((
                    Position(proj_pos),
                    Velocity(proj_vel),
                    Zone(zone.0),
                    Projectile {
                        damage: weapon.damage,
                        owner: req.entity_id,
                        spawn_time: current_time,
                        lifespan_seconds: 2.0,
                        rewind_ticks,
                    },
                    Collider { radius: 5.0 },
                ))
                .id();

            registry.register(new_id, new_ecs_entity);

            combat_events.spawn_projectiles.push(SpawnProjectileEvent {
                entity_id: new_id,
                position: proj_pos,
                velocity: proj_vel,
            });
        }
    }
}

/// Expire old projectiles.
pub fn projectile_expiration_system(
    mut commands: Commands,
    mut registry: ResMut<EntityRegistry>,
    time: Res<TickCounter>,
    config: Res<SimConfig>,
    query: Query<(Entity, &Projectile)>,
) {
    let current_time = time.tick as f64 * config.dt as f64;

    for (entity, proj) in query.iter() {
        if current_time >= proj.spawn_time + proj.lifespan_seconds as f64 {
            if let Some(id) = registry.by_entity.get(&entity).copied() {
                registry.unregister(id);
            }
            commands.entity(entity).despawn();
        }
    }
}

/// Detect projectile hits.
#[allow(clippy::type_complexity)]
pub fn hit_detection_system(
    mut commands: Commands,
    mut registry: ResMut<EntityRegistry>,
    mut combat_events: ResMut<CombatEvents>,
    projectiles: Query<(Entity, &Projectile, &Position, &Collider, &Zone)>,
    mut targets: Query<
        (
            Entity,
            &mut Health,
            &Position,
            Option<&PositionHistory>,
            &Collider,
            &Zone,
            &mut Inventory,
            Option<&Player>,
        ),
        Without<Dead>,
    >,
    tick: Res<TickCounter>,
    mut vaults: ResMut<FactionVaults>,
) {
    combat_events.health_changes.clear();
    combat_events.oxygen_changes.clear();
    combat_events.deaths.clear();
    combat_events.inventory_updates.clear();
    combat_events.credits_changes.clear();

    for (proj_ent, proj, proj_pos, proj_col, proj_zone) in projectiles.iter() {
        let mut hit = false;
        let rewind_tick = tick.tick.saturating_sub(proj.rewind_ticks as u64);

        for (
            target_ent,
            mut target_health,
            target_pos,
            target_history,
            target_col,
            target_zone,
            mut target_inv,
            target_player,
        ) in targets.iter_mut()
        {
            if proj_zone.0 != target_zone.0 {
                continue;
            }
            if let Some(&target_id) = registry.by_entity.get(&target_ent) {
                if target_id == proj.owner {
                    continue;
                }

                let rewound_pos = if proj.rewind_ticks == 0 {
                    target_pos.0
                } else if let Some(history) = target_history {
                    history
                        .samples
                        .iter()
                        .rev()
                        .find(|(sample_tick, _)| *sample_tick <= rewind_tick)
                        .map(|(_, p)| *p)
                        .unwrap_or(target_pos.0)
                } else {
                    target_pos.0
                };

                let dist_sq = (proj_pos.0 - rewound_pos).length_squared();
                let combined_radius = proj_col.radius + target_col.radius;

                if dist_sq <= combined_radius * combined_radius {
                    target_health.current -= proj.damage;
                    hit = true;

                    combat_events.health_changes.push(HealthChangedEvent {
                        entity_id: target_id,
                        health: target_health.current,
                        max_health: target_health.max,
                    });

                    if target_health.current <= 0.0 {
                        target_health.current = 0.0;
                        commands.entity(target_ent).insert(Dead);
                        combat_events.deaths.push(PlayerDiedEvent {
                            entity_id: target_id,
                        });

                        if let Some(player) = target_player
                            && let Some(faction_id) = player.faction_id
                        {
                            let bal = vaults.balances.entry(faction_id).or_insert(0);
                            *bal = (*bal).saturating_sub(25);
                            combat_events.credits_changes.push(
                                crate::resources::CreditsChangedEvent {
                                    faction_id,
                                    balance: *bal,
                                },
                            );
                        }

                        // Drop all inventory items
                        let mut offset = 0.0;
                        for item in target_inv.items.drain(..) {
                            let drop_pos = rewound_pos + lithos_protocol::Vec2::new(offset, offset);
                            offset += 10.0; // simple spread

                            let item_id = registry.next_entity_id();
                            let item_ent = commands
                                .spawn((
                                    Position(drop_pos),
                                    Velocity(lithos_protocol::Vec2::ZERO),
                                    Zone(target_zone.0),
                                    Collider { radius: 6.0 },
                                    Item { item_type: item },
                                ))
                                .id();
                            registry.register(item_id, item_ent);
                        }

                        // Notify client of empty inventory
                        combat_events.inventory_updates.push(InventoryUpdatedEvent {
                            entity_id: target_id,
                            items_json: "[]".to_string(),
                        });
                    }
                    break;
                }
            }
        }

        if hit {
            if let Some(id) = registry.by_entity.get(&proj_ent).copied() {
                registry.unregister(id);
            }
            commands.entity(proj_ent).despawn();
        }
    }
}

/// Computes power generation and powers consumers in the same zone.
pub fn power_grid_system(
    generators: Query<(&Zone, &crate::components::PowerGenerator)>,
    mut consumers: Query<(&Zone, &mut crate::components::PowerConsumer)>,
) {
    let mut zone_power: std::collections::HashMap<lithos_protocol::ZoneId, f32> =
        std::collections::HashMap::new();

    for (zone, generator) in generators.iter() {
        if generator.fuel_remaining > 0.0 {
            *zone_power.entry(zone.0).or_insert(0.0) += generator.output_kw;
        }
    }

    for (zone, mut consumer) in consumers.iter_mut() {
        if let Some(available) = zone_power.get_mut(&zone.0) {
            if *available >= consumer.required_kw {
                *available -= consumer.required_kw;
                consumer.is_powered = true;
            } else {
                consumer.is_powered = false;
            }
        } else {
            consumer.is_powered = false;
        }
    }
}

/// Consumes oxygen if unpowered, deals damage if empty.
#[allow(clippy::type_complexity)]
pub fn life_support_system(
    mut commands: Commands,
    mut combat_events: ResMut<CombatEvents>,
    mut registry: ResMut<EntityRegistry>,
    mut players: Query<
        (
            Entity,
            &Zone,
            &Position,
            &mut crate::components::Oxygen,
            &mut Health,
            &mut Inventory,
        ),
        (With<Player>, Without<Dead>),
    >,
    life_supports: Query<(
        &Zone,
        &crate::components::LifeSupport,
        &crate::components::PowerConsumer,
    )>,
) {
    for (entity, p_zone, pos, mut o2, mut health, mut inventory) in players.iter_mut() {
        // Overworld space has no oxygen, but players have spacesuits. Asteroid bases need life support.
        if matches!(p_zone.0, lithos_protocol::ZoneId::Overworld) {
            o2.current = o2.max;
            continue;
        }

        let mut has_life_support = false;
        for (ls_zone, _, consumer) in life_supports.iter() {
            if ls_zone.0 == p_zone.0 && consumer.is_powered {
                has_life_support = true;
                break;
            }
        }

        let prev_o2 = o2.current;
        if has_life_support {
            o2.current = (o2.current + 1.0).min(o2.max);
        } else {
            o2.current -= 0.5; // deplete O2
            if o2.current <= 0.0 {
                o2.current = 0.0;
                health.current -= 5.0; // asphyxiation damage

                if let Some(&id) = registry.by_entity.get(&entity) {
                    combat_events.health_changes.push(HealthChangedEvent {
                        entity_id: id,
                        health: health.current,
                        max_health: health.max,
                    });
                }

                if health.current <= 0.0 {
                    health.current = 0.0;
                    commands.entity(entity).insert(Dead);
                    if let Some(&id) = registry.by_entity.get(&entity) {
                        combat_events.deaths.push(PlayerDiedEvent { entity_id: id });

                        // Drop all inventory items.
                        let mut offset = 0.0;
                        for item in inventory.items.drain(..) {
                            let drop_pos = pos.0 + lithos_protocol::Vec2::new(offset, offset);
                            offset += 10.0;
                            let item_id = registry.next_entity_id();
                            let item_ent = commands
                                .spawn((
                                    Position(drop_pos),
                                    Velocity(lithos_protocol::Vec2::ZERO),
                                    Zone(p_zone.0),
                                    Collider { radius: 6.0 },
                                    Item { item_type: item },
                                ))
                                .id();
                            registry.register(item_id, item_ent);
                        }
                        combat_events.inventory_updates.push(InventoryUpdatedEvent {
                            entity_id: id,
                            items_json: "[]".to_string(),
                        });
                    }
                }
            }
        }

        // Emit oxygen change if value changed.
        if (o2.current - prev_o2).abs() > f32::EPSILON
            && let Some(&id) = registry.by_entity.get(&entity)
        {
            combat_events.oxygen_changes.push(crate::resources::OxygenChangedEvent {
                entity_id: id,
                current: o2.current,
                max: o2.max,
            });
        }
    }
}

/// Scrapper Dispenser default loadout items.
const SCRAPPER_LOADOUT: &[&str] = &["mining_laser", "scrap", "scrap"];
/// Cooldown between free loadouts (5 minutes at 20 TPS).
const LOADOUT_COOLDOWN_TICKS: u64 = 5 * 60 * 20;

/// Process respawn requests.
#[allow(clippy::type_complexity)]
pub fn respawn_system(
    mut commands: Commands,
    mut input_queue: ResMut<InputQueue>,
    registry: Res<EntityRegistry>,
    tick: Res<TickCounter>,
    mut combat_events: ResMut<CombatEvents>,
    mut query: Query<
        (
            Entity,
            &mut Health,
            &mut Position,
            &mut Zone,
            &mut Inventory,
            Option<&LastLoadoutTick>,
        ),
        With<Dead>,
    >,
) {
    for req in input_queue.respawns.drain(..) {
        if let Some(&ecs_entity) = registry.by_id.get(&req.entity_id)
            && let Ok((entity, mut health, mut pos, mut zone, mut inventory, last_loadout)) =
                query.get_mut(ecs_entity)
        {
            health.current = health.max;
            pos.0 = lithos_protocol::Vec2::ZERO; // Respawn at origin for now
            zone.0 = lithos_protocol::ZoneId::Overworld; // Send back to Overworld
            commands.entity(entity).remove::<Dead>();

            // Grant Scrapper Dispenser loadout if cooldown has expired.
            let can_loadout = match last_loadout {
                Some(ll) => tick.tick >= ll.tick + LOADOUT_COOLDOWN_TICKS,
                None => true,
            };

            if can_loadout {
                for item in SCRAPPER_LOADOUT {
                    inventory.items.push(item.to_string());
                }
                commands
                    .entity(entity)
                    .insert(LastLoadoutTick { tick: tick.tick });
                combat_events.inventory_updates.push(InventoryUpdatedEvent {
                    entity_id: req.entity_id,
                    items_json: format!(
                        "[{}]",
                        inventory
                            .items
                            .iter()
                            .map(|s| format!("\"{}\"", s))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
            }
        }
    }
}

/// Process item pickups when players collide with items.
#[allow(clippy::type_complexity)]
pub fn item_pickup_system(
    mut commands: Commands,
    mut registry: ResMut<EntityRegistry>,
    mut combat_events: ResMut<CombatEvents>,
    mut players: Query<
        (Entity, &mut Inventory, &Position, &Collider, &Zone),
        (With<Player>, Without<Dead>),
    >,
    items: Query<(Entity, &Item, &Position, &Collider, &Zone)>,
) {
    for (player_ent, mut player_inv, player_pos, player_col, player_zone) in players.iter_mut() {
        if let Some(&player_id) = registry.by_entity.get(&player_ent) {
            for (item_ent, item, item_pos, item_col, item_zone) in items.iter() {
                if player_zone.0 != item_zone.0 {
                    continue;
                }

                let dist_sq = (player_pos.0 - item_pos.0).length_squared();
                let combined_radius = player_col.radius + item_col.radius;

                if dist_sq <= combined_radius * combined_radius {
                    // Pick up item!
                    player_inv.items.push(item.item_type.clone());

                    combat_events.inventory_updates.push(InventoryUpdatedEvent {
                        entity_id: player_id,
                        // quick JSON string building for MVP
                        items_json: format!(
                            "[{}]",
                            player_inv
                                .items
                                .iter()
                                .map(|s| format!("\"{}\"", s))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    });

                    if let Some(id) = registry.by_entity.get(&item_ent).copied() {
                        registry.unregister(id);
                    }
                    commands.entity(item_ent).despawn();
                }
            }
        }
    }
}

/// Basic Automata AI logic.
#[allow(clippy::type_complexity)]
pub fn npc_ai_system(
    config: Res<SimConfig>,
    mut npcs: Query<(&mut Npc, &mut Velocity, &Position), Without<Dead>>,
    players: Query<&Position, (With<Player>, Without<Dead>)>,
) {
    let speed = config.max_speed * 0.5;

    for (mut npc, mut vel, pos) in npcs.iter_mut() {
        let mut nearest_dist_sq = f32::MAX;
        let mut nearest_pos = None;

        for player_pos in players.iter() {
            let dist_sq = (pos.0 - player_pos.0).length_squared();
            if dist_sq < nearest_dist_sq {
                nearest_dist_sq = dist_sq;
                nearest_pos = Some(player_pos.0);
            }
        }

        if let Some(target) = nearest_pos {
            if nearest_dist_sq < 1000.0 * 1000.0 && npc.npc_type == NpcType::Hostile {
                // Aggro: chase player
                npc.state = NpcState::Aggro;
                let dir = (target - pos.0).normalize();
                vel.0 = dir * speed;
            } else {
                // Patrol: return to spawn
                npc.state = NpcState::Patrol;
                let dist_to_spawn = (npc.spawn_pos - pos.0).length_squared();
                if dist_to_spawn > 100.0 {
                    let dir = (npc.spawn_pos - pos.0).normalize();
                    vel.0 = dir * speed * 0.5;
                } else {
                    vel.0 = lithos_protocol::Vec2::ZERO;
                }
            }
        } else {
            // Patrol: return to spawn
            npc.state = NpcState::Patrol;
            let dist_to_spawn = (npc.spawn_pos - pos.0).length_squared();
            if dist_to_spawn > 100.0 {
                let dir = (npc.spawn_pos - pos.0).normalize();
                vel.0 = dir * speed * 0.5;
            } else {
                vel.0 = lithos_protocol::Vec2::ZERO;
            }
        }
    }
}

/// Simulate NPC trader supply/demand and refresh quotes over time.
pub fn trader_market_system(
    mut market: ResMut<TraderMarket>,
    tick: Res<TickCounter>,
    registry: Res<EntityRegistry>,
    traders: Query<(Entity, &Npc), Without<Dead>>,
) {
    if market.quotes.is_empty() {
        for (entity, npc) in traders.iter() {
            if npc.npc_type != NpcType::Trader {
                continue;
            }

            let Some(&trader_entity_id) = registry.by_entity.get(&entity) else {
                continue;
            };

            for (item, base_price) in [
                ("iron", 10.0_f32),
                ("titanium", 22.0_f32),
                ("lithos", 80.0_f32),
                ("medkit", 45.0_f32),
            ] {
                market.quotes.push(crate::economy::TraderMarketState {
                    trader_entity_id,
                    item: item.to_string(),
                    base_price,
                    demand_scalar: 1.0,
                    available_credits: 2_500,
                });
            }
        }
    }

    if !tick.tick.is_multiple_of(400) {
        return;
    }

    for quote in &mut market.quotes {
        let cycle = ((tick.tick / 400) + quote.trader_entity_id.0) % 7;
        let sold_to_trader = cycle as i32;
        let bought_from_trader = (6 - cycle) as i32;
        quote.apply_daily_volume(sold_to_trader, bought_from_trader);
        quote.available_credits = (quote.available_credits
            + i64::from(bought_from_trader * 20 - sold_to_trader * 10))
        .clamp(500, 10_000);
    }
}

fn xp_to_next_level(level: u32) -> u32 {
    100 + (level.saturating_sub(1) * 50)
}

/// Process mining requests: check tool, find target, extract resources.
#[allow(clippy::type_complexity)]
pub fn mining_system(
    mut mine_queue: ResMut<MineQueue>,
    mut mining_events: ResMut<MiningEvents>,
    registry: Res<EntityRegistry>,
    mut players: Query<(&mut Inventory, &Position, &Zone), (With<Player>, Without<Dead>)>,
    mut resources: Query<(Entity, &mut ResourceNode, &Position, &Zone), Without<Dead>>,
) {
    mining_events.events.clear();
    mining_events.depleted.clear();

    const MINING_RANGE: f32 = 150.0;
    const XP_PER_UNIT: u32 = 5;

    for req in mine_queue.requests.drain(..) {
        let Some(&miner_ecs) = registry.by_id.get(&req.entity_id) else {
            continue;
        };

        let Ok((mut inventory, miner_pos, miner_zone)) = players.get_mut(miner_ecs) else {
            continue;
        };

        // Must have a mining laser in inventory.
        if !inventory.items.iter().any(|item| item == "mining_laser") {
            continue;
        }

        // Find target resource node.
        let mut target: Option<(bevy_ecs::entity::Entity, f32)> = None;

        if let Some(tid) = req.target_entity_id {
            // Explicit target.
            if let Some(&t_ecs) = registry.by_id.get(&tid)
                && let Ok((_, _, t_pos, t_zone)) = resources.get(t_ecs)
                && t_zone.0 == miner_zone.0
            {
                let dist_sq = (miner_pos.0 - t_pos.0).length_squared();
                if dist_sq <= MINING_RANGE * MINING_RANGE {
                    target = Some((t_ecs, dist_sq));
                }
            }
        }

        // If no explicit target or out of range, find nearest.
        if target.is_none() {
            let mut nearest_dist_sq = MINING_RANGE * MINING_RANGE;
            let mut nearest: Option<bevy_ecs::entity::Entity> = None;

            for (res_ent, _res_node, res_pos, res_zone) in resources.iter_mut() {
                if res_zone.0 != miner_zone.0 {
                    continue;
                }
                let dist_sq = (miner_pos.0 - res_pos.0).length_squared();
                if dist_sq < nearest_dist_sq {
                    nearest_dist_sq = dist_sq;
                    nearest = Some(res_ent);
                }
            }

            if let Some(n) = nearest {
                target = Some((n, nearest_dist_sq));
            }
        }

        let Some((target_ecs, _)) = target else {
            continue;
        };

        let Ok((_, mut node, _, _)) = resources.get_mut(target_ecs) else {
            continue;
        };

        if node.yield_amount == 0 {
            continue;
        }

        // Extract 1 unit per tick.
        node.yield_amount -= 1;
        let item_name = match node.resource_type {
            ResourceType::Iron => "iron",
            ResourceType::Titanium => "titanium",
            ResourceType::Lithos => "lithos",
        };
        inventory.items.push(item_name.to_string());

        if let Some(&res_id) = registry.by_entity.get(&target_ecs) {
            mining_events.events.push(MiningEvent {
                miner_entity_id: req.entity_id,
                resource_entity_id: res_id,
                item_gained: item_name.to_string(),
                amount: 1,
                xp_gained: XP_PER_UNIT,
            });
        }

        if node.yield_amount == 0
            && let Some(&res_id) = registry.by_entity.get(&target_ecs)
        {
            mining_events.depleted.push(res_id);
        }
    }
}

/// Process trade requests: validate, update inventories, and update faction vaults.
#[allow(clippy::type_complexity)]
pub fn trade_system(
    mut trade_queue: ResMut<crate::resources::TradeQueue>,
    mut trade_events: ResMut<crate::resources::TradeEvents>,
    mut market: ResMut<crate::resources::TraderMarket>,
    mut vaults: ResMut<crate::resources::FactionVaults>,
    registry: Res<crate::resources::EntityRegistry>,
    mut players: Query<(&mut Inventory, &Position, &Zone, &Player), (With<Player>, Without<Dead>)>,
    traders: Query<(Entity, &Npc, &Position, &Zone), Without<Dead>>,
) {
    trade_events.events.clear();
    trade_events.failures.clear();

    const TRADE_RANGE: f32 = 200.0;

    for req in trade_queue.requests.drain(..) {
        let Some(&player_ecs) = registry.by_id.get(&req.entity_id) else {
            continue;
        };

        let Ok((mut inventory, player_pos, player_zone, player)) = players.get_mut(player_ecs)
        else {
            continue;
        };

        let Some(faction_id) = player.faction_id else {
            trade_events
                .failures
                .push((req.entity_id, "no faction affiliation".to_string()));
            continue;
        };

        // Find nearest trader.
        let mut nearest_trader: Option<(
            lithos_protocol::EntityId,
            &crate::economy::TraderMarketState,
        )> = None;
        let mut nearest_dist_sq = TRADE_RANGE * TRADE_RANGE;

        for (trader_ent, npc, trader_pos, trader_zone) in traders.iter() {
            if npc.npc_type != NpcType::Trader {
                continue;
            }
            if trader_zone.0 != player_zone.0 {
                continue;
            }
            let dist_sq = (player_pos.0 - trader_pos.0).length_squared();
            if dist_sq >= nearest_dist_sq {
                continue;
            }
            if let Some(&tid) = registry.by_entity.get(&trader_ent)
                && let Some(quote) = market
                    .quotes
                    .iter()
                    .find(|q| q.trader_entity_id == tid && q.item == req.item)
            {
                nearest_dist_sq = dist_sq;
                nearest_trader = Some((tid, quote));
            }
        }

        let Some((trader_id, quote)) = nearest_trader else {
            trade_events
                .failures
                .push((req.entity_id, "no trader nearby".to_string()));
            continue;
        };

        if req.is_sell {
            // Sell: player gives item, faction gets credits.
            let mut owned = 0u32;
            for item in &inventory.items {
                if item == &req.item {
                    owned += 1;
                }
            }
            if owned < req.quantity {
                trade_events
                    .failures
                    .push((req.entity_id, "insufficient items".to_string()));
                continue;
            }

            let total_price = (quote.as_quote().buy_price * req.quantity as f32) as i64;
            if quote.available_credits < total_price {
                trade_events
                    .failures
                    .push((req.entity_id, "trader lacks credits".to_string()));
                continue;
            }

            // Remove items.
            let mut removed = 0u32;
            inventory.items.retain(|item| {
                if removed < req.quantity && item == &req.item {
                    removed += 1;
                    false
                } else {
                    true
                }
            });

            // Update market and vault.
            if let Some(q) = market
                .quotes
                .iter_mut()
                .find(|q| q.trader_entity_id == trader_id && q.item == req.item)
            {
                q.available_credits -= total_price;
                q.demand_scalar = (q.demand_scalar - 0.02).clamp(0.4, 2.2);
            }
            let bal = vaults.balances.entry(faction_id).or_insert(0);
            *bal = bal.saturating_add(total_price);

            trade_events.events.push(crate::resources::TradeEvent {
                entity_id: req.entity_id,
                item: req.item.clone(),
                quantity: req.quantity,
                total_price,
                is_sell: true,
            });
        } else {
            // Buy: player gets item, faction loses credits.
            let total_price = (quote.as_quote().sell_price * req.quantity as f32) as i64;
            let bal = vaults.balances.entry(faction_id).or_insert(0);
            if *bal < total_price {
                trade_events
                    .failures
                    .push((req.entity_id, "insufficient faction credits".to_string()));
                continue;
            }

            *bal = bal.saturating_sub(total_price);

            // Update market.
            if let Some(q) = market
                .quotes
                .iter_mut()
                .find(|q| q.trader_entity_id == trader_id && q.item == req.item)
            {
                q.available_credits += total_price;
                q.demand_scalar = (q.demand_scalar + 0.02).clamp(0.4, 2.2);
            }

            for _ in 0..req.quantity {
                inventory.items.push(req.item.clone());
            }

            trade_events.events.push(crate::resources::TradeEvent {
                entity_id: req.entity_id,
                item: req.item.clone(),
                quantity: req.quantity,
                total_price,
                is_sell: false,
            });
        }
    }
}

/// Apply queued XP gains and emit progression updates for clients.
pub fn progression_system(
    mut queue: ResMut<ProgressionQueue>,
    mut mining_events: ResMut<MiningEvents>,
    mut combat_events: ResMut<CombatEvents>,
    registry: Res<EntityRegistry>,
    mut players: Query<&mut Progression, Without<Dead>>,
) {
    combat_events.progression_updates.clear();

    // Convert mining events into Extraction XP.
    for event in mining_events.events.drain(..) {
        queue.gains.push(XpGainRequest {
            entity_id: event.miner_entity_id,
            branch: lithos_protocol::SkillBranch::Extraction,
            amount: event.xp_gained,
        });
    }

    for gain in queue.gains.drain(..) {
        let Some(&ecs_entity) = registry.by_id.get(&gain.entity_id) else {
            continue;
        };
        let Ok(mut progression) = players.get_mut(ecs_entity) else {
            continue;
        };
        let Some(branch) = progression.branches.get_mut(&gain.branch) else {
            continue;
        };

        branch.xp = branch.xp.saturating_add(gain.amount);
        while branch.xp >= branch.xp_to_next {
            branch.xp -= branch.xp_to_next;
            branch.level = branch.level.saturating_add(1);
            branch.xp_to_next = xp_to_next_level(branch.level);
        }

        let mut branches = Vec::new();
        for (skill, state) in &progression.branches {
            branches.push(lithos_protocol::ProgressionSnapshot {
                branch: *skill,
                level: state.level,
                xp: state.xp,
                xp_to_next: state.xp_to_next,
            });
        }

        combat_events
            .progression_updates
            .push(ProgressionUpdatedEvent {
                entity_id: gain.entity_id,
                branches,
            });
    }
}

/// Drive dynamic world event lifecycle (meteor showers, solar flares, POIs).
pub fn dynamic_events_system(
    mut active: ResMut<ActiveDynamicEvents>,
    mut bus: ResMut<DynamicEventBus>,
    tick: Res<TickCounter>,
) {
    bus.started.clear();
    bus.ended_event_ids.clear();

    if active.next_id == 0 {
        active.next_id = 1;
    }

    if tick.tick.is_multiple_of(900) {
        let kind_index = (tick.tick / 900) % 3;
        let kind = match kind_index {
            0 => lithos_protocol::DynamicEventKind::MeteorShower,
            1 => lithos_protocol::DynamicEventKind::SolarFlare,
            _ => lithos_protocol::DynamicEventKind::CrashedFreighter,
        };
        let description = match kind {
            lithos_protocol::DynamicEventKind::MeteorShower => {
                "Meteor shower detected in the Mid-Zone".to_string()
            }
            lithos_protocol::DynamicEventKind::SolarFlare => {
                "Solar flare causing sensor disruption".to_string()
            }
            lithos_protocol::DynamicEventKind::CrashedFreighter => {
                "Crashed freighter beacon spotted in the Core".to_string()
            }
        };
        let event = crate::resources::DynamicEventState {
            event_id: active.next_id,
            kind,
            started_at_tick: tick.tick,
            expires_at_tick: tick.tick + 300,
            description,
        };
        active.next_id += 1;
        active.active.push(event.clone());
        bus.started.push(event);
    }

    let mut retained = Vec::with_capacity(active.active.len());
    for event in active.active.drain(..) {
        if tick.tick >= event.expires_at_tick {
            bus.ended_event_ids.push(event.event_id);
        } else {
            retained.push(event);
        }
    }
    active.active = retained;
}

/// Advance raid warnings into active breaches and close finished raids.
pub fn raid_state_system(
    mut raids: ResMut<RaidStateStore>,
    mut bus: ResMut<RaidEventBus>,
    tick: Res<TickCounter>,
) {
    bus.warnings.clear();
    bus.started.clear();
    bus.ended.clear();

    let mut retained = Vec::with_capacity(raids.raids.len());
    for mut raid in raids.raids.drain(..) {
        if !raid.breach_active && tick.tick >= raid.warning_ends_at_tick {
            raid.breach_active = true;
            bus.started.push(raid.clone());
        }

        if raid.breach_active && tick.tick >= raid.breach_ends_at_tick {
            bus.ended.push((raid, false));
        } else {
            retained.push(raid);
        }
    }
    raids.raids = retained;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::*;
    use crate::resources::*;
    use lithos_protocol::{EntityId, PlayerId, Vec2, ZoneId};

    /// Helper to create a minimal ECS world with resources.
    fn setup_world() -> World {
        let mut world = World::new();
        world.insert_resource(SimConfig::default());
        world.insert_resource(TickCounter::default());
        world.insert_resource(InputQueue::default());
        world.insert_resource(LastProcessedSeq::default());
        world.insert_resource(EntityRegistry::default());
        world.insert_resource(ZoneChangeEvents::default());
        world.insert_resource(CombatEvents::default());
        world
    }

    fn spawn_player(world: &mut World, entity_id: EntityId) -> bevy_ecs::entity::Entity {
        let ecs_entity = world
            .spawn((
                Position(Vec2::ZERO),
                Velocity(Vec2::ZERO),
                PositionHistory::default(),
                Progression::default(),
                Player {
                    id: PlayerId::new(),
                    auth_subject: None,
                    faction_id: None,
                },
                Zone(ZoneId::Overworld),
            ))
            .id();

        let mut registry = world.resource_mut::<EntityRegistry>();
        registry.register(entity_id, ecs_entity);
        ecs_entity
    }

    #[test]
    fn test_movement_applies_velocity() {
        let mut world = setup_world();
        let eid = EntityId(1);
        let ecs_entity = spawn_player(&mut world, eid);

        // Set velocity directly.
        world
            .entity_mut(ecs_entity)
            .get_mut::<Velocity>()
            .unwrap()
            .0 = Vec2::new(100.0, 0.0);

        // Run movement system.
        let mut schedule = Schedule::default();
        schedule.add_systems(movement_system);
        schedule.run(&mut world);

        let pos = world.entity(ecs_entity).get::<Position>().unwrap();
        // At 20 TPS, dt = 0.05, so position = 100 * 0.05 = 5.0
        assert!((pos.0.x - 5.0).abs() < 0.01);
        assert!((pos.0.y).abs() < 0.01);
    }

    #[test]
    fn test_bounds_clamp_position() {
        let mut world = setup_world();
        let eid = EntityId(2);
        let ecs_entity = spawn_player(&mut world, eid);

        // Place entity far outside bounds.
        world
            .entity_mut(ecs_entity)
            .get_mut::<Position>()
            .unwrap()
            .0 = Vec2::new(9999.0, -9999.0);

        let mut schedule = Schedule::default();
        schedule.add_systems(bounds_system);
        schedule.run(&mut world);

        let pos = world.entity(ecs_entity).get::<Position>().unwrap();
        let half = 2000.0;
        assert!((pos.0.x - half).abs() < 0.01);
        assert!((pos.0.y - (-half)).abs() < 0.01);
    }

    #[test]
    fn test_input_processing() {
        let mut world = setup_world();
        let eid = EntityId(3);
        spawn_player(&mut world, eid);

        // Queue a movement input.
        world.resource_mut::<InputQueue>().moves.push(MoveInput {
            entity_id: eid,
            direction: Vec2::new(1.0, 0.0),
            seq: 1,
        });

        let mut schedule = Schedule::default();
        schedule.add_systems(process_inputs_system);
        schedule.run(&mut world);

        // Check last processed seq was updated.
        let last_seq = world.resource::<LastProcessedSeq>();
        assert_eq!(last_seq.map.get(&eid), Some(&1));
    }

    #[test]
    fn test_zone_transfer() {
        let mut world = setup_world();
        let eid = EntityId(4);
        let ecs_entity = spawn_player(&mut world, eid);

        // Queue a zone transfer.
        world
            .resource_mut::<InputQueue>()
            .zone_transfers
            .push(ZoneTransferRequest {
                entity_id: eid,
                target: ZoneId::AsteroidBase(1),
            });

        let mut schedule = Schedule::default();
        schedule.add_systems(zone_transfer_system);
        schedule.run(&mut world);

        let zone = world.entity(ecs_entity).get::<Zone>().unwrap();
        assert_eq!(zone.0, ZoneId::AsteroidBase(1));
    }
}
