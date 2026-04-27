//! ECS resources — global state stored in the bevy_ecs World.

use bevy_ecs::prelude::*;
use std::collections::HashMap;

use lithos_protocol::{EntityId, PlayerId, ProgressionSnapshot, Vec2, ZoneId};

/// Configuration constants for the game simulation.
#[derive(Resource, Debug, Clone)]
pub struct SimConfig {
    /// Fixed timestep in seconds (1 / tick_rate).
    pub dt: f32,
    /// Maximum player movement speed in units/second.
    pub max_speed: f32,
    /// World bounds (half-extents from origin).
    pub world_half_size: f32,
    /// Number of historical ticks retained for lag compensation.
    pub lag_comp_history_ticks: u64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            dt: 1.0 / 20.0,          // 20 TPS
            max_speed: 200.0,        // units/sec
            world_half_size: 2000.0, // 4000x4000 world
            lag_comp_history_ticks: 64,
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

/// A pending fire request from a client.
#[derive(Debug, Clone)]
pub struct FireRequest {
    pub entity_id: EntityId,
    pub direction: Vec2,
    pub client_latency_ms: u16,
}

/// A pending respawn request from a client.
#[derive(Debug, Clone)]
pub struct RespawnRequest {
    pub entity_id: EntityId,
}

/// Queue of client inputs to be processed this tick.
#[derive(Resource, Debug, Default)]
pub struct InputQueue {
    pub moves: Vec<MoveInput>,
    pub zone_transfers: Vec<ZoneTransferRequest>,
    pub fires: Vec<FireRequest>,
    pub respawns: Vec<RespawnRequest>,
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

#[derive(Debug, Clone)]
pub struct SpawnProjectileEvent {
    pub entity_id: EntityId,
    pub position: Vec2,
    pub velocity: Vec2,
}

#[derive(Debug, Clone)]
pub struct HealthChangedEvent {
    pub entity_id: EntityId,
    pub health: f32,
    pub max_health: f32,
}

#[derive(Debug, Clone)]
pub struct PlayerDiedEvent {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone)]
pub struct InventoryUpdatedEvent {
    pub entity_id: EntityId,
    pub items_json: String,
}

#[derive(Debug, Clone)]
pub struct CreditsChangedEvent {
    pub faction_id: u64,
    pub balance: i64,
}

#[derive(Debug, Clone)]
pub struct ProgressionUpdatedEvent {
    pub entity_id: EntityId,
    pub branches: Vec<ProgressionSnapshot>,
}

/// Combat-related events emitted this tick.
#[derive(Resource, Debug, Default)]
pub struct CombatEvents {
    pub spawn_projectiles: Vec<SpawnProjectileEvent>,
    pub health_changes: Vec<HealthChangedEvent>,
    pub deaths: Vec<PlayerDiedEvent>,
    pub inventory_updates: Vec<InventoryUpdatedEvent>,
    pub credits_changes: Vec<CreditsChangedEvent>,
    pub progression_updates: Vec<ProgressionUpdatedEvent>,
}

/// Chat line staged by the server layer to fan-out.
#[derive(Debug, Clone)]
pub struct ChatEvent {
    pub from_entity_id: EntityId,
    pub channel: lithos_protocol::ChatChannel,
    pub text: String,
    pub sent_at_unix_ms: u64,
    pub faction_id: Option<u64>,
}

/// Chat messages emitted this tick.
#[derive(Resource, Debug, Default)]
pub struct ChatEvents {
    pub messages: Vec<ChatEvent>,
}

/// Active dynamic world event.
#[derive(Debug, Clone)]
pub struct DynamicEventState {
    pub event_id: u64,
    pub kind: lithos_protocol::DynamicEventKind,
    pub started_at_tick: u64,
    pub expires_at_tick: u64,
    pub description: String,
}

/// Lifecycle records for dynamic events.
#[derive(Resource, Debug, Default)]
pub struct DynamicEventBus {
    pub started: Vec<DynamicEventState>,
    pub ended_event_ids: Vec<u64>,
}

/// Tracks currently active dynamic events.
#[derive(Resource, Debug, Default)]
pub struct ActiveDynamicEvents {
    pub next_id: u64,
    pub active: Vec<DynamicEventState>,
}

/// Raid lifecycle state.
#[derive(Debug, Clone)]
pub struct RaidState {
    pub attacker_faction_id: u64,
    pub defender_faction_id: u64,
    pub warning_ends_at_tick: u64,
    pub breach_ends_at_tick: u64,
    pub breach_active: bool,
}

/// Active raids in the shard.
#[derive(Resource, Debug, Default)]
pub struct RaidStateStore {
    pub raids: Vec<RaidState>,
}

/// Raid transition bus for server broadcasting.
#[derive(Resource, Debug, Default)]
pub struct RaidEventBus {
    pub warnings: Vec<RaidState>,
    pub started: Vec<RaidState>,
    pub ended: Vec<(RaidState, bool)>,
}

/// A queued progression XP gain.
#[derive(Debug, Clone)]
pub struct XpGainRequest {
    pub entity_id: EntityId,
    pub branch: lithos_protocol::SkillBranch,
    pub amount: u32,
}

/// Progression queue owned by server layer / ECS systems.
#[derive(Resource, Debug, Default)]
pub struct ProgressionQueue {
    pub gains: Vec<XpGainRequest>,
}

/// Trader market simulation state.
#[derive(Resource, Debug, Default)]
pub struct TraderMarket {
    pub quotes: Vec<crate::economy::TraderMarketState>,
}

/// Credits balances per faction (Faction Vaults).
#[derive(Resource, Debug, Default)]
pub struct FactionVaults {
    pub balances: HashMap<u64, i64>,
}
