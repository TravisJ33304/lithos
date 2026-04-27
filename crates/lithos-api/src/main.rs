//! # lithos-api
//!
//! Central Orchestration API for Lithos.

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use base64::Engine;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use lithos_protocol::{
    FactionMembership, LeaderboardEntry, ServerListing, SkillBranch, SkillBranch::*,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone)]
struct ApiConfig {
    bind_addr: String,
    database_url: String,
    supabase_jwks_url: Option<String>,
    supabase_jwt_issuer: Option<String>,
    supabase_jwt_audience: Option<String>,
    admin_api_key: Option<String>,
    heartbeat_api_key: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_addr: std::env::var("API_BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/lithos".to_string()),
            supabase_jwks_url: std::env::var("SUPABASE_JWKS_URL").ok(),
            supabase_jwt_issuer: std::env::var("SUPABASE_JWT_ISSUER").ok(),
            supabase_jwt_audience: std::env::var("SUPABASE_JWT_AUDIENCE").ok(),
            admin_api_key: std::env::var("API_ADMIN_KEY").ok(),
            heartbeat_api_key: std::env::var("CENTRAL_API_KEY").ok(),
        }
    }
}

#[derive(Debug, Clone)]
struct AppState {
    pool: sqlx::PgPool,
    config: ApiConfig,
    server_registry: Arc<RwLock<HashMap<String, ServerRecord>>>,
}

#[derive(Debug, Clone)]
struct ServerRecord {
    listing: ServerListing,
}

#[derive(Debug, Clone)]
struct AuthContext {
    user_id: String,
    username: String,
}

#[derive(Debug, Deserialize)]
struct RawClaims {
    sub: String,
    #[allow(dead_code)]
    exp: usize,
    #[serde(default)]
    preferred_username: Option<String>,
    #[serde(default)]
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: Option<String>,
    kty: String,
    n: Option<String>,
    e: Option<String>,
    x5c: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct ProfileResponse {
    user_id: String,
    username: String,
    faction: Option<FactionMembership>,
    progression: Vec<ProgressionApiRow>,
}

#[derive(Debug, Serialize)]
struct ProgressionApiRow {
    branch: SkillBranch,
    level: i32,
    xp: i32,
    xp_to_next: i32,
}

#[derive(Debug, Deserialize)]
struct UpsertProfileRequest {
    username: Option<String>,
}

#[derive(Debug, Serialize)]
struct FactionResponse {
    faction_id: i64,
    name: String,
    wealth: i64,
    members: Vec<FactionMemberResponse>,
}

#[derive(Debug, Serialize)]
struct FactionMemberResponse {
    user_id: String,
    username: String,
    role: String,
}

#[derive(Debug, Deserialize)]
struct CreateFactionRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct AddFactionMemberRequest {
    user_id: String,
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateFactionRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct AdminWipeRequest {
    archive_label: Option<String>,
    new_world_seed: Option<u32>,
}

#[derive(Debug, Serialize)]
struct AdminWipeResponse {
    archive_label: String,
    new_world_seed: u32,
    archived_rows: i64,
}

#[derive(Debug, Deserialize)]
struct HeartbeatRequest {
    server_id: String,
    name: String,
    websocket_url: String,
    region: String,
    population: u32,
    capacity: u32,
    healthy: bool,
    world_seed: u32,
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn require_header(headers: &HeaderMap, name: &str) -> Result<String, (StatusCode, String)> {
    let Some(raw) = headers.get(name) else {
        return Err((StatusCode::UNAUTHORIZED, format!("missing header: {name}")));
    };
    let value = raw
        .to_str()
        .map_err(|_| (StatusCode::UNAUTHORIZED, format!("invalid header: {name}")))?;
    Ok(value.to_string())
}

fn normalize_username(name: &str) -> String {
    let mut cleaned = String::with_capacity(name.len().min(24));
    for ch in name.chars() {
        if cleaned.len() >= 24 {
            break;
        }
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            cleaned.push(ch);
        }
    }
    if cleaned.is_empty() {
        "guest".to_string()
    } else {
        cleaned
    }
}

async fn decode_supabase_claims(token: &str, config: &ApiConfig) -> Result<RawClaims> {
    let jwks_url = config
        .supabase_jwks_url
        .as_deref()
        .context("SUPABASE_JWKS_URL is not configured")?;

    let header = decode_header(token).context("failed to decode JWT header")?;
    let kid = header.kid.clone().context("JWT missing kid header")?;

    let jwks = reqwest::Client::new()
        .get(jwks_url)
        .send()
        .await
        .context("failed to fetch JWKS")?
        .error_for_status()
        .context("JWKS endpoint returned non-success status")?
        .json::<Jwks>()
        .await
        .context("failed to parse JWKS payload")?;

    let key = jwks
        .keys
        .into_iter()
        .find(|k| k.kid.as_deref() == Some(kid.as_str()))
        .context("matching key id not found in JWKS")?;

    let decoding_key = if key.kty == "RSA" {
        let n = key.n.context("JWKS RSA key missing modulus")?;
        let e = key.e.context("JWKS RSA key missing exponent")?;
        DecodingKey::from_rsa_components(&n, &e).context("invalid JWKS RSA key")?
    } else if key.kty == "EC" {
        let x5c = key.x5c.context("JWKS EC key missing x5c")?;
        let cert = x5c.first().context("JWKS EC x5c chain empty")?;
        let cert_der = base64::engine::general_purpose::STANDARD
            .decode(cert)
            .context("invalid x5c certificate")?;
        DecodingKey::from_ec_der(&cert_der)
    } else {
        anyhow::bail!("unsupported JWK key type: {}", key.kty)
    };

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_required_spec_claims(&["sub", "exp"]);
    if let Some(issuer) = config.supabase_jwt_issuer.as_deref() {
        validation.set_issuer(&[issuer]);
    }
    if let Some(aud) = config.supabase_jwt_audience.as_deref() {
        validation.set_audience(&[aud]);
    }

    let decoded = decode::<RawClaims>(token, &decoding_key, &validation)
        .context("JWT validation failed")?;
    Ok(decoded.claims)
}

async fn require_user_auth(headers: &HeaderMap, state: &AppState) -> Result<AuthContext, (StatusCode, String)> {
    let authz = require_header(headers, "authorization")?;
    let token = authz
        .strip_prefix("Bearer ")
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "expected Bearer token".to_string()))?;

    let claims = decode_supabase_claims(token, &state.config)
        .await
        .map_err(|err| (StatusCode::UNAUTHORIZED, format!("auth failed: {err}")))?;

    let username = claims
        .preferred_username
        .or_else(|| claims.email.and_then(|email| email.split('@').next().map(ToOwned::to_owned)))
        .map(|u| normalize_username(&u))
        .unwrap_or_else(|| {
            let short = claims.sub.chars().take(8).collect::<String>();
            format!("pilot-{short}")
        });

    Ok(AuthContext {
        user_id: claims.sub,
        username,
    })
}

fn require_api_key(
    headers: &HeaderMap,
    expected: Option<&str>,
    name: &str,
) -> Result<(), (StatusCode, String)> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let value = require_header(headers, name)?;
    if value != expected {
        return Err((StatusCode::UNAUTHORIZED, "invalid api key".to_string()));
    }
    Ok(())
}

async fn ensure_schema(pool: &sqlx::PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS player_profiles (
            user_id TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            faction_id BIGINT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS factions (
            id BIGSERIAL PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            wealth BIGINT NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS faction_members (
            faction_id BIGINT NOT NULL REFERENCES factions(id) ON DELETE CASCADE,
            user_id TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'member',
            PRIMARY KEY (faction_id, user_id)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS progression (
            user_id TEXT NOT NULL,
            branch TEXT NOT NULL,
            level INT NOT NULL DEFAULT 1,
            xp INT NOT NULL DEFAULT 0,
            xp_to_next INT NOT NULL DEFAULT 100,
            PRIMARY KEY (user_id, branch)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS overworld_wipes (
            id BIGSERIAL PRIMARY KEY,
            archive_label TEXT NOT NULL,
            world_seed INT NOT NULL,
            archived_rows BIGINT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse { status: "ok" })
}

async fn upsert_profile(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<UpsertProfileRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let auth = require_user_auth(&headers, &state).await?;
    let username = payload
        .username
        .as_deref()
        .map(normalize_username)
        .unwrap_or(auth.username.clone());

    sqlx::query(
        "INSERT INTO player_profiles (user_id, username) VALUES ($1, $2)
         ON CONFLICT (user_id)
         DO UPDATE SET username = EXCLUDED.username, updated_at = NOW()",
    )
    .bind(&auth.user_id)
    .bind(&username)
    .execute(&state.pool)
    .await
    .map_err(internal_err)?;

    for branch in [Fabrication, Extraction, Ballistics, Cybernetics] {
        sqlx::query(
            "INSERT INTO progression (user_id, branch, level, xp, xp_to_next)
             VALUES ($1, $2, 1, 0, 100)
             ON CONFLICT (user_id, branch) DO NOTHING",
        )
        .bind(&auth.user_id)
        .bind(format!("{branch:?}"))
        .execute(&state.pool)
        .await
        .map_err(internal_err)?;
    }

    Ok((StatusCode::OK, Json(serde_json::json!({ "ok": true }))))
}

async fn get_profile(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let auth = require_user_auth(&headers, &state).await?;

    let profile = sqlx::query(
        "SELECT p.user_id, p.username, p.faction_id, f.name AS faction_name, fm.role
         FROM player_profiles p
         LEFT JOIN factions f ON p.faction_id = f.id
         LEFT JOIN faction_members fm ON fm.faction_id = p.faction_id AND fm.user_id = p.user_id
         WHERE p.user_id = $1",
    )
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_err)?;

    let Some(profile) = profile else {
        return Err((StatusCode::NOT_FOUND, "profile not found".to_string()));
    };

    let progression_rows = sqlx::query(
        "SELECT branch, level, xp, xp_to_next FROM progression WHERE user_id = $1 ORDER BY branch",
    )
    .bind(&auth.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_err)?;

    let mut progression = Vec::with_capacity(progression_rows.len());
    for row in progression_rows {
        let branch_name = row.try_get::<String, _>("branch").map_err(internal_err)?;
        let branch = parse_branch(&branch_name).unwrap_or(Fabrication);
        progression.push(ProgressionApiRow {
            branch,
            level: row.try_get("level").map_err(internal_err)?,
            xp: row.try_get("xp").map_err(internal_err)?,
            xp_to_next: row.try_get("xp_to_next").map_err(internal_err)?,
        });
    }

    let faction_id = profile
        .try_get::<Option<i64>, _>("faction_id")
        .map_err(internal_err)?;
    let faction_name = profile
        .try_get::<Option<String>, _>("faction_name")
        .map_err(internal_err)?;
    let role = profile
        .try_get::<Option<String>, _>("role")
        .map_err(internal_err)?;

    let faction = faction_id.map(|id| FactionMembership {
        faction_id: id as u64,
        faction_name: faction_name.unwrap_or_else(|| "Unknown".to_string()),
        role: role.unwrap_or_else(|| "member".to_string()),
    });

    Ok(Json(ProfileResponse {
        user_id: profile.try_get("user_id").map_err(internal_err)?,
        username: profile.try_get("username").map_err(internal_err)?,
        faction,
        progression,
    }))
}

async fn create_faction(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreateFactionRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let auth = require_user_auth(&headers, &state).await?;
    let name = payload.name.trim();
    if name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "faction name cannot be empty".to_string()));
    }

    let mut tx = state.pool.begin().await.map_err(internal_err)?;

    let row = sqlx::query("INSERT INTO factions (name) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(&mut *tx)
        .await
        .map_err(internal_err)?;
    let faction_id = row.try_get::<i64, _>("id").map_err(internal_err)?;

    sqlx::query(
        "INSERT INTO faction_members (faction_id, user_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(faction_id)
    .bind(&auth.user_id)
    .execute(&mut *tx)
    .await
    .map_err(internal_err)?;

    sqlx::query("UPDATE player_profiles SET faction_id = $1 WHERE user_id = $2")
        .bind(faction_id)
        .bind(&auth.user_id)
        .execute(&mut *tx)
        .await
        .map_err(internal_err)?;

    tx.commit().await.map_err(internal_err)?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({ "faction_id": faction_id }))))
}

async fn list_factions(State(state): State<AppState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let factions = sqlx::query("SELECT id, name, wealth FROM factions ORDER BY name")
        .fetch_all(&state.pool)
        .await
        .map_err(internal_err)?;

    let members = sqlx::query(
        "SELECT fm.faction_id, fm.user_id, fm.role, COALESCE(pp.username, fm.user_id) AS username
         FROM faction_members fm
         LEFT JOIN player_profiles pp ON pp.user_id = fm.user_id",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(internal_err)?;

    let mut members_by_faction: HashMap<i64, Vec<FactionMemberResponse>> = HashMap::new();
    for row in members {
        let faction_id = row.try_get::<i64, _>("faction_id").map_err(internal_err)?;
        members_by_faction
            .entry(faction_id)
            .or_default()
            .push(FactionMemberResponse {
                user_id: row.try_get("user_id").map_err(internal_err)?,
                username: row.try_get("username").map_err(internal_err)?,
                role: row.try_get("role").map_err(internal_err)?,
            });
    }

    let mut response = Vec::with_capacity(factions.len());
    for row in factions {
        let faction_id = row.try_get::<i64, _>("id").map_err(internal_err)?;
        response.push(FactionResponse {
            faction_id,
            name: row.try_get("name").map_err(internal_err)?,
            wealth: row.try_get("wealth").map_err(internal_err)?,
            members: members_by_faction.remove(&faction_id).unwrap_or_default(),
        });
    }

    Ok(Json(response))
}

async fn update_faction(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(faction_id): Path<i64>,
    Json(payload): Json<UpdateFactionRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let auth = require_user_auth(&headers, &state).await?;
    ensure_owner(&state.pool, faction_id, &auth.user_id).await?;

    sqlx::query("UPDATE factions SET name = $1 WHERE id = $2")
        .bind(payload.name.trim())
        .bind(faction_id)
        .execute(&state.pool)
        .await
        .map_err(internal_err)?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "ok": true }))))
}

async fn delete_faction(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(faction_id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let auth = require_user_auth(&headers, &state).await?;
    ensure_owner(&state.pool, faction_id, &auth.user_id).await?;

    let mut tx = state.pool.begin().await.map_err(internal_err)?;
    sqlx::query("UPDATE player_profiles SET faction_id = NULL WHERE faction_id = $1")
        .bind(faction_id)
        .execute(&mut *tx)
        .await
        .map_err(internal_err)?;
    sqlx::query("DELETE FROM factions WHERE id = $1")
        .bind(faction_id)
        .execute(&mut *tx)
        .await
        .map_err(internal_err)?;
    tx.commit().await.map_err(internal_err)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn add_faction_member(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(faction_id): Path<i64>,
    Json(payload): Json<AddFactionMemberRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let auth = require_user_auth(&headers, &state).await?;
    ensure_owner(&state.pool, faction_id, &auth.user_id).await?;

    let role = payload.role.unwrap_or_else(|| "member".to_string());
    sqlx::query(
        "INSERT INTO faction_members (faction_id, user_id, role) VALUES ($1, $2, $3)
         ON CONFLICT (faction_id, user_id) DO UPDATE SET role = EXCLUDED.role",
    )
    .bind(faction_id)
    .bind(&payload.user_id)
    .bind(&role)
    .execute(&state.pool)
    .await
    .map_err(internal_err)?;

    sqlx::query(
        "INSERT INTO player_profiles (user_id, username, faction_id) VALUES ($1, $2, $3)
         ON CONFLICT (user_id) DO UPDATE SET faction_id = EXCLUDED.faction_id",
    )
    .bind(&payload.user_id)
    .bind(format!("pilot-{}", &payload.user_id.chars().take(8).collect::<String>()))
    .bind(faction_id)
    .execute(&state.pool)
    .await
    .map_err(internal_err)?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "ok": true }))))
}

async fn remove_faction_member(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((faction_id, user_id)): Path<(i64, String)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let auth = require_user_auth(&headers, &state).await?;
    ensure_owner(&state.pool, faction_id, &auth.user_id).await?;

    sqlx::query("DELETE FROM faction_members WHERE faction_id = $1 AND user_id = $2")
        .bind(faction_id)
        .bind(&user_id)
        .execute(&state.pool)
        .await
        .map_err(internal_err)?;
    sqlx::query("UPDATE player_profiles SET faction_id = NULL WHERE user_id = $1 AND faction_id = $2")
        .bind(&user_id)
        .bind(faction_id)
        .execute(&state.pool)
        .await
        .map_err(internal_err)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn list_servers(State(state): State<AppState>) -> impl IntoResponse {
    let stale_after_ms = Duration::from_secs(45).as_millis() as u64;
    let now = now_unix_ms();

    let mut registry = state.server_registry.write().await;
    registry.retain(|_, record| now.saturating_sub(record.listing.last_heartbeat_unix_ms) < stale_after_ms);

    let mut listings = registry
        .values()
        .map(|record| {
            let mut listing = record.listing.clone();
            listing.healthy = listing.healthy
                && now.saturating_sub(listing.last_heartbeat_unix_ms) < stale_after_ms;
            listing
        })
        .collect::<Vec<_>>();
    listings.sort_by(|a, b| a.region.cmp(&b.region).then(a.name.cmp(&b.name)));

    Json(listings)
}

async fn heartbeat_server(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<HeartbeatRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    require_api_key(
        &headers,
        state.config.heartbeat_api_key.as_deref(),
        "x-api-key",
    )?;

    let listing = ServerListing {
        server_id: payload.server_id.clone(),
        name: payload.name,
        websocket_url: payload.websocket_url,
        region: payload.region,
        population: payload.population,
        capacity: payload.capacity,
        healthy: payload.healthy,
        last_heartbeat_unix_ms: now_unix_ms(),
    };

    tracing::debug!(
        server_id = %payload.server_id,
        world_seed = payload.world_seed,
        population = payload.population,
        "received server heartbeat"
    );

    state.server_registry.write().await.insert(
        payload.server_id,
        ServerRecord { listing },
    );

    Ok((StatusCode::OK, Json(serde_json::json!({ "ok": true }))))
}

async fn get_leaderboard(State(state): State<AppState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let rows = sqlx::query("SELECT id, name, wealth FROM factions ORDER BY wealth DESC, name ASC LIMIT 50")
        .fetch_all(&state.pool)
        .await
        .map_err(internal_err)?;

    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        entries.push(LeaderboardEntry {
            faction_id: row.try_get::<i64, _>("id").map_err(internal_err)? as u64,
            faction_name: row.try_get("name").map_err(internal_err)?,
            wealth: row.try_get("wealth").map_err(internal_err)?,
        });
    }

    Ok(Json(entries))
}

async fn admin_wipe(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<AdminWipeRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    require_api_key(
        &headers,
        state.config.admin_api_key.as_deref(),
        "x-admin-key",
    )?;

    let archive_label = payload
        .archive_label
        .unwrap_or_else(|| format!("wipe-{}", now_unix_ms()));
    let new_seed = payload
        .new_world_seed
        .unwrap_or_else(|| rand::random::<u32>().max(1));

    let mut tx = state.pool.begin().await.map_err(internal_err)?;

    let archived_rows = sqlx::query(
        "WITH reset_players AS (
            UPDATE players
            SET x = 0, y = 0, zone_id = 'overworld', health = 100, inventory = '[]'
            RETURNING 1
        )
        SELECT COUNT(*) AS c FROM reset_players",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(internal_err)?
    .try_get::<i64, _>("c")
    .map_err(internal_err)?;

    sqlx::query("TRUNCATE TABLE base_structures")
        .execute(&mut *tx)
        .await
        .map_err(internal_err)?;

    sqlx::query(
        "INSERT INTO overworld_wipes (archive_label, world_seed, archived_rows)
         VALUES ($1, $2, $3)",
    )
    .bind(&archive_label)
    .bind(new_seed as i32)
    .bind(archived_rows)
    .execute(&mut *tx)
    .await
    .map_err(internal_err)?;

    tx.commit().await.map_err(internal_err)?;

    Ok(Json(AdminWipeResponse {
        archive_label,
        new_world_seed: new_seed,
        archived_rows,
    }))
}

async fn ensure_owner(pool: &sqlx::PgPool, faction_id: i64, user_id: &str) -> Result<(), (StatusCode, String)> {
    let row = sqlx::query(
        "SELECT role FROM faction_members WHERE faction_id = $1 AND user_id = $2",
    )
    .bind(faction_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(internal_err)?;

    let Some(row) = row else {
        return Err((StatusCode::FORBIDDEN, "not a faction member".to_string()));
    };
    let role = row.try_get::<String, _>("role").map_err(internal_err)?;
    if role != "owner" {
        return Err((StatusCode::FORBIDDEN, "owner role required".to_string()));
    }
    Ok(())
}

fn parse_branch(name: &str) -> Option<SkillBranch> {
    match name {
        "Fabrication" => Some(Fabrication),
        "Extraction" => Some(Extraction),
        "Ballistics" => Some(Ballistics),
        "Cybernetics" => Some(Cybernetics),
        _ => None,
    }
}

fn internal_err<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("lithos=info".parse()?))
        .init();

    let config = ApiConfig::default();
    let pool = sqlx::PgPool::connect(&config.database_url)
        .await
        .context("failed to connect to postgres")?;
    ensure_schema(&pool).await?;

    let state = AppState {
        pool,
        config: config.clone(),
        server_registry: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/profile", get(get_profile).post(upsert_profile))
        .route("/v1/factions", get(list_factions).post(create_faction))
        .route("/v1/factions/{faction_id}", patch(update_faction).delete(delete_faction))
        .route(
            "/v1/factions/{faction_id}/members",
            post(add_faction_member),
        )
        .route(
            "/v1/factions/{faction_id}/members/{user_id}",
            delete(remove_faction_member),
        )
        .route("/v1/servers", get(list_servers))
        .route("/v1/leaderboard/factions", get(get_leaderboard))
        .route("/v1/admin/overworld/wipe", post(admin_wipe))
        .route("/internal/servers/heartbeat", post(heartbeat_server))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.bind_addr))?;
    tracing::info!(addr = %listener.local_addr()?, "lithos-api listening");

    axum::serve(listener, app).await?;
    Ok(())
}
