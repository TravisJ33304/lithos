//! # lithos-world
//!
//! ECS components, systems, and world generation for the Lithos game.
//!
//! This crate contains all game logic that is shared between the dedicated
//! game server and any tooling (map editors, bot simulations). It uses
//! [`bevy_ecs`] as a standalone ECS — no Bevy App or renderer involved.

pub mod components;
pub mod crafting;
pub mod resources;
pub mod simulation;
pub mod systems;
pub mod world_gen;
