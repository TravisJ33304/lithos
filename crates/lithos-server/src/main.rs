//! # lithos-server
//!
//! Dedicated Game Server for Lithos.
//!
//! Runs a fixed-tick game loop using [`bevy_ecs`] and accepts player
//! connections over WebSockets via [`tokio_tungstenite`].

mod auth;
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
    /// Central API URL for heartbeats.
    pub central_api_url: String,
    /// Optional shared secret used for server heartbeat auth.
    pub central_api_key: Option<String>,
    /// Public server ID for browser registration.
    pub server_id: String,
    /// Display name for server browser.
    pub server_name: String,
    /// Region code for server browser.
    pub region: String,
    /// Public websocket URL advertised to clients.
    pub websocket_public_url: String,
    /// Supabase JWKS URL for JWT verification.
    pub supabase_jwks_url: Option<String>,
    /// Expected JWT issuer (optional).
    pub supabase_jwt_issuer: Option<String>,
    /// Expected JWT audience (optional).
    pub supabase_jwt_audience: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:9001".to_string(),
            tick_rate: 20,
            max_players: 100,
            world_seed: 12345,
            db_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgresql://postgres:postgres@localhost:5432/lithos".to_string()
            }),
            central_api_url: std::env::var("CENTRAL_API_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string()),
            central_api_key: std::env::var("CENTRAL_API_KEY").ok(),
            server_id: std::env::var("SERVER_ID")
                .unwrap_or_else(|_| format!("srv-{}", uuid::Uuid::new_v4())),
            server_name: std::env::var("SERVER_NAME")
                .unwrap_or_else(|_| "Lithos Dev Shard".to_string()),
            region: std::env::var("SERVER_REGION").unwrap_or_else(|_| "local".to_string()),
            websocket_public_url: std::env::var("WS_PUBLIC_URL")
                .unwrap_or_else(|_| "ws://127.0.0.1:9001".to_string()),
            supabase_jwks_url: std::env::var("SUPABASE_JWKS_URL").ok(),
            supabase_jwt_issuer: std::env::var("SUPABASE_JWT_ISSUER").ok(),
            supabase_jwt_audience: std::env::var("SUPABASE_JWT_AUDIENCE").ok(),
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
        );",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS base_structures (
            id SERIAL PRIMARY KEY,
            zone_id VARCHAR(50) NOT NULL,
            tile_type VARCHAR(50) NOT NULL,
            grid_x INT NOT NULL,
            grid_y INT NOT NULL,
            UNIQUE(zone_id, grid_x, grid_y)
        );",
    )
    .execute(&pool)
    .await?;

    tracing::info!(
        addr = %config.listen_addr,
        tick_rate = config.tick_rate,
        server_id = %config.server_id,
        server_name = %config.server_name,
        region = %config.region,
        "lithos-server starting"
    );

    game_loop::run(config, pool).await
}
