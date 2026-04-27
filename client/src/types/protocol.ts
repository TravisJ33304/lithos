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
	| { Pong: { client_timestamp: number; server_timestamp: number } }
	| { Disconnect: { reason: string } };
