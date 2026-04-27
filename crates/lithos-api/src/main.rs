//! # lithos-api
//!
//! Central Orchestration API for Lithos.
//!
//! Provides REST endpoints for player authentication (via Supabase JWTs),
//! faction management, server browser, and the Faction Wealth Leaderboard.

use axum::{Router, routing::get};
use tracing_subscriber::EnvFilter;

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("lithos=info".parse()?))
        .init();

    let app = Router::new().route("/health", get(health));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("lithos-api listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
