//! ECS components for the Lithos game world.

use bevy_ecs::prelude::*;
use lithos_protocol::{PlayerId, Vec2, ZoneId};

/// The position of an entity in the game world.
#[derive(Component, Debug, Clone, Copy)]
pub struct Position(pub Vec2);

/// The velocity of an entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity(pub Vec2);

/// Marks an entity as a player and stores their ID.
#[derive(Component, Debug, Clone, Copy)]
pub struct Player {
    pub id: PlayerId,
}

/// Which zone an entity currently belongs to.
#[derive(Component, Debug, Clone, Copy)]
pub struct Zone(pub ZoneId);
