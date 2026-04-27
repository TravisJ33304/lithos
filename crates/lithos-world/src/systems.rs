//! ECS systems — game logic that runs each tick.

use bevy_ecs::prelude::*;

use crate::components::{Position, Velocity, Zone};
use crate::resources::{InputQueue, LastProcessedSeq, SimConfig, TickCounter, EntityRegistry, ZoneChangeEvent, ZoneChangeEvents};

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
    mut query: Query<&mut Velocity>,
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
pub fn movement_system(config: Res<SimConfig>, mut query: Query<(&mut Position, &Velocity)>) {
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
    mut query: Query<&mut Zone>,
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
