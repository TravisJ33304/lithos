//! # lithos-server
//!
//! Dedicated Game Server for Lithos.
//!
//! Runs a fixed-tick game loop using [`bevy_ecs`] and accepts player
//! connections over WebSockets via [`tokio_tungstenite`].

mod connection;
mod game_loop;
mod network;

use tracing_subscriber::EnvFilter;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// WebSocket listen address.
    pub listen_addr: String,
    /// Game tick rate (ticks per second).
    pub tick_rate: u32,
    /// Maximum concurrent players.
    pub max_players: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:9001".to_string(),
            tick_rate: 20,
            max_players: 100,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("lithos=info".parse()?))
        .init();

    let config = ServerConfig::default();
    tracing::info!(addr = %config.listen_addr, tick_rate = config.tick_rate, "lithos-server starting");

    game_loop::run(config).await
}
