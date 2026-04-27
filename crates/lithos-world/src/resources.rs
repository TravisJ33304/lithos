//! ECS resources — global state stored in the bevy_ecs World.

use bevy_ecs::prelude::*;
use std::collections::HashMap;

use lithos_protocol::{EntityId, PlayerId, Vec2, ZoneId};

/// Configuration constants for the game simulation.
#[derive(Resource, Debug, Clone)]
pub struct SimConfig {
    /// Fixed timestep in seconds (1 / tick_rate).
    pub dt: f32,
    /// Maximum player movement speed in units/second.
    pub max_speed: f32,
    /// World bounds (half-extents from origin).
    pub world_half_size: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            dt: 1.0 / 20.0,          // 20 TPS
            max_speed: 200.0,         // units/sec
            world_half_size: 2000.0,  // 4000x4000 world
        }
    }
}

/// Monotonically increasing tick counter.
#[derive(Resource, Debug, Default)]
pub struct TickCounter {
    pub tick: u64,
}

/// A pending movement input from a client.
#[derive(Debug, Clone)]
pub struct MoveInput {
    pub entity_id: EntityId,
    pub direction: Vec2,
    pub seq: u32,
}

/// A pending zone transfer request from a client.
#[derive(Debug, Clone)]
pub struct ZoneTransferRequest {
    pub entity_id: EntityId,
    pub target: ZoneId,
}

/// Queue of client inputs to be processed this tick.
#[derive(Resource, Debug, Default)]
pub struct InputQueue {
    pub moves: Vec<MoveInput>,
    pub zone_transfers: Vec<ZoneTransferRequest>,
}

/// Tracks the last processed input sequence per entity (for client reconciliation).
#[derive(Resource, Debug, Default)]
pub struct LastProcessedSeq {
    pub map: HashMap<EntityId, u32>,
}

/// Maps EntityId ↔ bevy_ecs Entity for stable ID lookup.
#[derive(Resource, Debug, Default)]
pub struct EntityRegistry {
    /// EntityId → bevy Entity
    pub by_id: HashMap<EntityId, bevy_ecs::entity::Entity>,
    /// bevy Entity → EntityId
    pub by_entity: HashMap<bevy_ecs::entity::Entity, EntityId>,
    /// PlayerId → EntityId
    pub player_entities: HashMap<PlayerId, EntityId>,
    /// Next entity ID to assign.
    next_id: u64,
}

impl EntityRegistry {
    /// Allocate a new unique EntityId.
    pub fn next_entity_id(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Register a mapping between an EntityId and a bevy Entity.
    pub fn register(&mut self, id: EntityId, entity: bevy_ecs::entity::Entity) {
        self.by_id.insert(id, entity);
        self.by_entity.insert(entity, id);
    }

    /// Unregister an entity.
    pub fn unregister(&mut self, id: EntityId) {
        if let Some(entity) = self.by_id.remove(&id) {
            self.by_entity.remove(&entity);
        }
    }
}

/// A completed zone change event, emitted by the zone_transfer_system.
#[derive(Debug, Clone)]
pub struct ZoneChangeEvent {
    pub entity_id: EntityId,
    pub new_zone: ZoneId,
}

/// Events emitted this tick that need to be sent to clients.
#[derive(Resource, Debug, Default)]
pub struct ZoneChangeEvents {
    pub events: Vec<ZoneChangeEvent>,
}
