//! ECS systems — game logic that runs each tick.

use bevy_ecs::prelude::*;

use crate::components::{Position, Velocity, Zone, Health, Weapon, Projectile, Collider, Dead, Player, Inventory, Item};
use crate::resources::{InputQueue, LastProcessedSeq, SimConfig, TickCounter, EntityRegistry, ZoneChangeEvent, ZoneChangeEvents, CombatEvents, SpawnProjectileEvent, HealthChangedEvent, PlayerDiedEvent, InventoryUpdatedEvent};

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

/// Apply velocity to position (Euler integration).
pub fn movement_system(config: Res<SimConfig>, mut query: Query<(&mut Position, &Velocity), Without<Dead>>) {
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
                    if dir.x == 0.0 && dir.y == 0.0 { continue; }
                    
                    let proj_vel = dir * weapon.projectile_speed;
                    // Spawn projectile slightly ahead
                    let proj_pos = pos.0 + dir * 20.0;
                    
                    let new_id = registry.next_entity_id();
                    let new_ecs_entity = commands.spawn((
                        Position(proj_pos),
                        Velocity(proj_vel),
                        Zone(zone.0),
                        Projectile {
                            damage: weapon.damage,
                            owner: req.entity_id,
                            spawn_time: current_time,
                            lifespan_seconds: 2.0,
                        },
                        Collider { radius: 5.0 },
                    )).id();
                    
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
    mut targets: Query<(Entity, &mut Health, &Position, &Collider, &Zone, &mut Inventory), Without<Dead>>,
) {
    combat_events.health_changes.clear();
    combat_events.deaths.clear();
    combat_events.inventory_updates.clear();

    for (proj_ent, proj, proj_pos, proj_col, proj_zone) in projectiles.iter() {
        let mut hit = false;
        
        for (target_ent, mut target_health, target_pos, target_col, target_zone, mut target_inv) in targets.iter_mut() {
            if proj_zone.0 != target_zone.0 { continue; }
            if let Some(&target_id) = registry.by_entity.get(&target_ent) {
                if target_id == proj.owner { continue; }
                
                let dist_sq = (proj_pos.0 - target_pos.0).length_squared();
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
                        
                        // Drop all inventory items
                        let mut offset = 0.0;
                        for item in target_inv.items.drain(..) {
                            let drop_pos = target_pos.0 + lithos_protocol::Vec2::new(offset, offset);
                            offset += 10.0; // simple spread
                            
                            let item_id = registry.next_entity_id();
                            let item_ent = commands.spawn((
                                Position(drop_pos),
                                Velocity(lithos_protocol::Vec2::ZERO),
                                Zone(target_zone.0),
                                Collider { radius: 6.0 },
                                Item { item_type: item },
                            )).id();
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

/// Process respawn requests.
pub fn respawn_system(
    mut commands: Commands,
    mut input_queue: ResMut<InputQueue>,
    registry: Res<EntityRegistry>,
    mut query: Query<(Entity, &mut Health, &mut Position, &mut Zone), With<Dead>>,
) {
    for req in input_queue.respawns.drain(..) {
        if let Some(&ecs_entity) = registry.by_id.get(&req.entity_id)
            && let Ok((entity, mut health, mut pos, mut zone)) = query.get_mut(ecs_entity)
        {
                health.current = health.max;
                pos.0 = lithos_protocol::Vec2::ZERO; // Respawn at origin for now
                zone.0 = lithos_protocol::ZoneId::Overworld; // Send back to Overworld
                commands.entity(entity).remove::<Dead>();
            }
        }
}

/// Process item pickups when players collide with items.
#[allow(clippy::type_complexity)]
pub fn item_pickup_system(
    mut commands: Commands,
    mut registry: ResMut<EntityRegistry>,
    mut combat_events: ResMut<CombatEvents>,
    mut players: Query<(Entity, &mut Inventory, &Position, &Collider, &Zone), (With<Player>, Without<Dead>)>,
    items: Query<(Entity, &Item, &Position, &Collider, &Zone)>,
) {
    for (player_ent, mut player_inv, player_pos, player_col, player_zone) in players.iter_mut() {
        if let Some(&player_id) = registry.by_entity.get(&player_ent) {
            for (item_ent, item, item_pos, item_col, item_zone) in items.iter() {
                if player_zone.0 != item_zone.0 { continue; }
                
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
                            player_inv.items.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", ")
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
        let ecs_entity = world.spawn((
            Position(Vec2::ZERO),
            Velocity(Vec2::ZERO),
            Player { id: PlayerId::new() },
            Zone(ZoneId::Overworld),
        )).id();

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
        world.entity_mut(ecs_entity).get_mut::<Velocity>().unwrap().0 = Vec2::new(100.0, 0.0);

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
        world.entity_mut(ecs_entity).get_mut::<Position>().unwrap().0 = Vec2::new(9999.0, -9999.0);

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
        world.resource_mut::<InputQueue>().zone_transfers.push(
            ZoneTransferRequest {
                entity_id: eid,
                target: ZoneId::AsteroidBase(1),
            },
        );

        let mut schedule = Schedule::default();
        schedule.add_systems(zone_transfer_system);
        schedule.run(&mut world);

        let zone = world.entity(ecs_entity).get::<Zone>().unwrap();
        assert_eq!(zone.0, ZoneId::AsteroidBase(1));
    }
}
