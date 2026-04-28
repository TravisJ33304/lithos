//! ECS components for the Lithos game world.

use bevy_ecs::prelude::*;
use lithos_protocol::{PlayerId, SkillBranch, Vec2, ZoneId};

/// The position of an entity in the game world.
#[derive(Component, Debug, Clone, Copy)]
pub struct Position(pub Vec2);

/// The velocity of an entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity(pub Vec2);

/// Marks an entity as a player and stores their ID.
#[derive(Component, Debug, Clone)]
pub struct Player {
    pub id: PlayerId,
    /// Optional auth subject from Supabase JWT `sub` claim.
    pub auth_subject: Option<String>,
    /// Optional faction affiliation for faction chat + raids.
    pub faction_id: Option<u64>,
}

/// Which zone an entity currently belongs to.
#[derive(Component, Debug, Clone, Copy)]
pub struct Zone(pub ZoneId);

/// Health of an entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

/// A weapon equipped by an entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct Weapon {
    pub damage: f32,
    pub projectile_speed: f32,
    pub cooldown_seconds: f32,
    pub last_fired_time: f64, // using standard seconds timestamp
}

/// Marks an entity as a projectile.
#[derive(Component, Debug, Clone, Copy)]
pub struct Projectile {
    pub damage: f32,
    pub owner: lithos_protocol::EntityId,
    pub spawn_time: f64,
    pub lifespan_seconds: f32,
    pub rewind_ticks: u32,
}

/// An inventory holding items. Simple list of strings for now.
#[derive(Component, Debug, Clone)]
pub struct Inventory {
    pub items: Vec<String>,
}

/// An item dropped in the world.
#[derive(Component, Debug, Clone)]
pub struct Item {
    pub item_type: String,
}

/// A simple circle collider for hit detection.
#[derive(Component, Debug, Clone, Copy)]
pub struct Collider {
    pub radius: f32,
}

/// Marks an entity as dead.
#[derive(Component, Debug, Clone, Copy)]
pub struct Dead;

/// AI state for an NPC.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcState {
    Patrol,
    Aggro,
    Attack,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcType {
    Hostile,
    Trader,
}

#[derive(Component, Debug, Clone)]
pub struct Npc {
    pub npc_type: NpcType,
    pub state: NpcState,
    pub target: Option<lithos_protocol::EntityId>,
    pub spawn_pos: lithos_protocol::Vec2,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub enum ResourceType {
    Iron,
    Titanium,
    Lithos,
}

#[derive(Component, Debug, Clone)]
pub struct ResourceNode {
    pub resource_type: ResourceType,
    pub yield_amount: u32,
}

// ── Base Building & Persistence ──────────────────────────────────────────────

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub enum TileType {
    Wall,
    Door,
    Workbench,
    Generator,
}

/// Marks an entity as a structure placed on a base grid.
#[derive(Component, Debug, Clone)]
pub struct BaseTile {
    pub tile_type: TileType,
    /// Grid coordinates (e.g. 1 unit = 40 world pixels)
    pub grid_x: i32,
    pub grid_y: i32,
}

/// A structure that generates power.
#[derive(Component, Debug, Clone)]
pub struct PowerGenerator {
    pub output_kw: f32,
    pub fuel_remaining: f32,
}

/// A structure that consumes power.
#[derive(Component, Debug, Clone)]
pub struct PowerConsumer {
    pub required_kw: f32,
    pub is_powered: bool,
}

/// A life support module that generates oxygen if powered.
#[derive(Component, Debug, Clone)]
pub struct LifeSupport {
    pub oxygen_output_per_tick: f32,
}

/// Player's oxygen level.
#[derive(Component, Debug, Clone)]
pub struct Oxygen {
    pub current: f32,
    pub max: f32,
}

/// Recent historical positions for lag compensation rewind.
#[derive(Component, Debug, Clone)]
pub struct PositionHistory {
    pub samples: std::collections::VecDeque<(u64, Vec2)>,
}

impl Default for PositionHistory {
    fn default() -> Self {
        Self {
            samples: std::collections::VecDeque::with_capacity(64),
        }
    }
}

/// Character progression state.
#[derive(Component, Debug, Clone)]
pub struct Progression {
    pub branches: std::collections::HashMap<SkillBranch, ProgressionBranchState>,
}

impl Default for Progression {
    fn default() -> Self {
        let mut branches = std::collections::HashMap::new();
        for branch in [
            SkillBranch::Fabrication,
            SkillBranch::Extraction,
            SkillBranch::Ballistics,
            SkillBranch::Cybernetics,
        ] {
            branches.insert(
                branch,
                ProgressionBranchState {
                    level: 1,
                    xp: 0,
                    xp_to_next: 100,
                },
            );
        }
        Self { branches }
    }
}

/// Per-branch progression values.
#[derive(Debug, Clone, Copy)]
pub struct ProgressionBranchState {
    pub level: u32,
    pub xp: u32,
    pub xp_to_next: u32,
}

/// Tracks when the player last received a Scrapper Dispenser loadout.
#[derive(Component, Debug, Clone, Copy)]
pub struct LastLoadoutTick {
    pub tick: u64,
}

/// Faction vault balance.
#[derive(Component, Debug, Clone, Copy)]
pub struct FactionVault {
    pub faction_id: u64,
    pub credits: i64,
}
