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
    /// World Generation Seed
    pub world_seed: u32,
    /// Database connection URL
    pub db_url: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:9001".to_string(),
            tick_rate: 20,
            max_players: 100,
            world_seed: 12345,
            db_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://postgres:s7Eqdd&KYQea&0^h433S@db.uxouuxrjaudnwlnbfyqz.supabase.co:5432/postgres".to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("lithos=info".parse()?))
        .init();

    let config = ServerConfig::default();
    
    tracing::info!("Connecting to database...");
    let pool = sqlx::PgPool::connect(&config.db_url).await?;
    tracing::info!("Connected to Postgres!");
    
    // Create necessary tables if they don't exist
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS players (
            id UUID PRIMARY KEY,
            username VARCHAR(255) NOT NULL,
            x FLOAT NOT NULL,
            y FLOAT NOT NULL,
            zone_id VARCHAR(50) NOT NULL,
            health FLOAT NOT NULL,
            inventory JSONB NOT NULL,
            last_login TIMESTAMPTZ DEFAULT NOW()
        );"
    )
    .execute(&pool)
    .await?;

    tracing::info!(addr = %config.listen_addr, tick_rate = config.tick_rate, "lithos-server starting");

    game_loop::run(config, pool).await
}
