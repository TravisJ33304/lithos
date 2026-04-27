//! # lithos-protocol
//!
//! Shared message types and codec for the Lithos game protocol.
//!
//! This crate defines the canonical [`ClientMessage`] and [`ServerMessage`] enums
//! that are serialized as MessagePack frames over WebSocket connections, plus
//! shared component types used by both the server ECS and the client's mirrored
//! TypeScript interfaces.

pub mod codec;
pub mod messages;
pub mod types;

pub use codec::{decode, encode};
pub use messages::{ClientMessage, ServerMessage};
pub use types::*;
