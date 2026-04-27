//! Shared data types used across the Lithos protocol.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a player entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub Uuid);

impl PlayerId {
    /// Create a new random player ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PlayerId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for an entity in the game world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

/// A 2D position in the game world.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Squared length of the vector (avoids a sqrt).
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Length (magnitude) of the vector.
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Dot product between two vectors.
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }

    /// Returns the unit vector, or `ZERO` if the length is near zero.
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len < 1e-6 {
            Self::ZERO
        } else {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        }
    }

    /// Clamps the vector's length to at most `max_len`.
    pub fn clamp_length(self, max_len: f32) -> Self {
        let len_sq = self.length_squared();
        if len_sq > max_len * max_len {
            self.normalize() * max_len
        } else {
            self
        }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

/// Identifies which zone an entity is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZoneId {
    /// The shared overworld (Zone 0).
    Overworld,
    /// A faction's private asteroid base.
    AsteroidBase(u32),
}

/// Describes the visual type of the entity for the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotEntityType {
    Player,
    Hostile,
    Trader,
    ResourceNode,
    Item,
    Projectile,
    Unknown,
}

/// Chat channel scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatChannel {
    Global,
    Faction,
}

/// Player progression branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillBranch {
    Fabrication,
    Extraction,
    Ballistics,
    Cybernetics,
}

/// Progression payload for one branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressionSnapshot {
    pub branch: SkillBranch,
    pub level: u32,
    pub xp: u32,
    pub xp_to_next: u32,
}

/// Dynamic event type broadcast by the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DynamicEventKind {
    MeteorShower,
    SolarFlare,
    CrashedFreighter,
}

/// Lightweight server-browser record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerListing {
    pub server_id: String,
    pub name: String,
    pub websocket_url: String,
    pub region: String,
    pub population: u32,
    pub capacity: u32,
    pub healthy: bool,
    pub last_heartbeat_unix_ms: u64,
}

/// Faction leaderboard row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub faction_id: u64,
    pub faction_name: String,
    pub wealth: i64,
}

/// Faction membership view for a player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionMembership {
    pub faction_id: u64,
    pub faction_name: String,
    pub role: String,
}

/// Dynamic event record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicEventSnapshot {
    pub event_id: u64,
    pub kind: DynamicEventKind,
    pub started_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub description: String,
}

/// Raid state snapshot for UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaidStateSnapshot {
    pub attacker_faction_id: u64,
    pub defender_faction_id: u64,
    pub warning_remaining_seconds: u32,
    pub breach_active: bool,
}

/// Trader market quote.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraderQuote {
    pub trader_entity_id: EntityId,
    pub item: String,
    pub buy_price: f32,
    pub sell_price: f32,
    pub demand_scalar: f32,
    pub available_credits: i64,
}

/// Snapshot of a single entity's state, sent from server to clients.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub id: EntityId,
    pub position: Vec2,
    pub velocity: Vec2,
    pub zone: ZoneId,
    pub entity_type: SnapshotEntityType,
}
