/**
 * Network message types — mirrors lithos-protocol Rust types.
 *
 * These interfaces MUST stay in sync with the Rust `ClientMessage` and
 * `ServerMessage` enums defined in `crates/lithos-protocol/src/messages.rs`.
 */

export interface Vec2 {
	x: number;
	y: number;
}

export type ZoneId = { Overworld: null } | { AsteroidBase: number };

export interface EntitySnapshot {
	id: number;
	position: Vec2;
	velocity: Vec2;
	zone: ZoneId;
}

// ── Client → Server ──────────────────────────────────────────────────

export type ClientMessage =
	| { Join: { token: string } }
	| { Move: { direction: Vec2; seq: number } }
	| { ZoneTransfer: { target: ZoneId } }
	| { Fire: { direction: Vec2 } }
	| "Respawn"
	| { Ping: { timestamp: number } };

// ── Server → Client ──────────────────────────────────────────────────

export type ServerMessage =
	| { JoinAck: { player_id: string; entity_id: number; zone: ZoneId } }
	| {
			StateSnapshot: {
				tick: number;
				last_processed_seq: number;
				entities: EntitySnapshot[];
			};
	  }
	| { ZoneChanged: { zone: ZoneId } }
	| { HealthChanged: { entity_id: number; health: number; max_health: number } }
	| { PlayerDied: { entity_id: number } }
	| { InventoryUpdated: { entity_id: number; items_json: string } }
	| { SpawnProjectile: { entity_id: number; position: Vec2; velocity: Vec2 } }
	| { Pong: { client_timestamp: number; server_timestamp: number } }
	| { Disconnect: { reason: string } };
