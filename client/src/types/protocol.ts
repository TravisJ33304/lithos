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

export type ChatChannel = "Global" | "Faction";

export type SkillBranch =
	| "Fabrication"
	| "Extraction"
	| "Ballistics"
	| "Cybernetics";

export type SnapshotEntityType =
	| "Player"
	| "Hostile"
	| "Rover"
	| "Drone"
	| "AssaultWalker"
	| "SniperWalker"
	| "HeavyFlamethrower"
	| "CoreWarden"
	| "Trader"
	| "ResourceNode"
	| "Item"
	| "Projectile"
	| "Unknown";

export type TerrainType =
	| "Empty"
	| "Rock"
	| "DeepRavine"
	| "AsteroidField"
	| "AutomataSpire";

export type CeilingType = "Open" | "Enclosed";

export interface TileData {
	terrain: TerrainType;
	ceiling: CeilingType;
	height: number;
}

export interface EntitySnapshot {
	id: number;
	position: Vec2;
	velocity: Vec2;
	zone: ZoneId;
	entity_type: SnapshotEntityType;
}

export interface ProgressionSnapshot {
	branch: SkillBranch;
	level: number;
	xp: number;
	xp_to_next: number;
}

export type DynamicEventKind =
	| "MeteorShower"
	| "SolarFlare"
	| "CrashedFreighter";

export interface ServerListing {
	server_id: string;
	name: string;
	websocket_url: string;
	region: string;
	population: number;
	capacity: number;
	healthy: boolean;
	last_heartbeat_unix_ms: number;
}

export interface LeaderboardEntry {
	faction_id: number;
	faction_name: string;
	wealth: number;
}

export interface FactionMembership {
	faction_id: number;
	faction_name: string;
	role: string;
}

export interface DynamicEventSnapshot {
	event_id: number;
	kind: DynamicEventKind;
	started_at_unix_ms: number;
	expires_at_unix_ms: number;
	description: string;
}

export interface RaidStateSnapshot {
	attacker_faction_id: number;
	defender_faction_id: number;
	warning_remaining_seconds: number;
	breach_active: boolean;
}

export interface TraderQuote {
	trader_entity_id: number;
	item: string;
	buy_price: number;
	sell_price: number;
	demand_scalar: number;
	available_credits: number;
	daily_credit_limit: number;
	daily_credits_used: number;
}

export type ItemRarity = "Common" | "Uncommon" | "Rare" | "Epic";

export type ItemCategory =
	| "Resource"
	| "Component"
	| "Consumable"
	| "Weapon"
	| "Tool"
	| "Structure"
	| "Utility";

export interface InventoryItemStack {
	item: string;
	quantity: number;
	rarity: ItemRarity;
	category: ItemCategory;
}

export interface InventorySnapshot {
	entity_id: number;
	items: InventoryItemStack[];
}

export interface ItemDefinition {
	item: string;
	display_name: string;
	description: string;
	rarity: ItemRarity;
	category: ItemCategory;
	stack_limit: number;
}

export interface RecipeDefinition {
	name: string;
	output: string;
	required_branch: SkillBranch;
	required_level: number;
	inputs: string[];
}

export type InteractableKind =
	| "ResourceNode"
	| "SalvageSite"
	| "LootContainer"
	| "HackingTarget"
	| "FabricationPlant"
	| "CommsArray"
	| "Trader";

export interface InteractableSnapshot {
	target_entity_id: number;
	kind: InteractableKind;
	required_tool: string | null;
	can_interact: boolean;
}

export interface PowerNetworkSnapshot {
	network_id: number;
	zone: ZoneId;
	generation_kw: number;
	load_kw: number;
	consumers_powered: number;
	consumers_total: number;
}

// ── Client → Server ──────────────────────────────────────────────────

export type ClientMessage =
	| { Join: { token: string } }
	| { Move: { direction: Vec2; seq: number } }
	| { ZoneTransfer: { target: ZoneId } }
	| { Fire: { direction: Vec2; client_latency_ms: number } }
	| "Respawn"
	| { Craft: { recipe: string } }
	| { BuildStructure: { item: string; grid_x: number; grid_y: number } }
	| { Ping: { timestamp: number } }
	| { Chat: { channel: ChatChannel; text: string } }
	| "RequestTraderQuotes"
	| { InitiateRaid: { defender_faction_id: number } }
	| { Mine: { target_entity_id: number | null } }
	| { SellItem: { item: string; quantity: number } }
	| { BuyItem: { item: string; quantity: number } }
	| { Interact: { target_entity_id: number | null } }
	| { AltFire: { direction: Vec2; client_latency_ms: number } }
	| { DropItem: { item: string; quantity: number } }
	| { UseItem: { item: string } }
	| { EquipItem: { item: string; slot: string } }
	| "RequestCraftingState"
	| "RequestPowerState"
	| { StartHack: { target_entity_id: number } }
	| "CancelHack"
	| "RequestRaidTargets";

// ── Server → Client ──────────────────────────────────────────────────

export type ServerMessage =
	| {
			JoinAck: {
				player_id: string;
				entity_id: number;
				zone: ZoneId;
				world_seed: number;
			};
	  }
	| {
			StateSnapshot: {
				tick: number;
				last_processed_seq: number;
				entities: EntitySnapshot[];
			};
	  }
	| { ZoneChanged: { zone: ZoneId } }
	| { HealthChanged: { entity_id: number; health: number; max_health: number } }
	| { OxygenChanged: { entity_id: number; current: number; max: number } }
	| { PlayerDied: { entity_id: number } }
	| { InventoryUpdated: { entity_id: number; items_json: string } }
	| { InventorySnapshot: { inventory: InventorySnapshot } }
	| { SpawnProjectile: { entity_id: number; position: Vec2; velocity: Vec2 } }
	| {
			ChatMessage: {
				from_entity_id: number;
				channel: ChatChannel;
				text: string;
				sent_at_unix_ms: number;
			};
	  }
	| { CreditsChanged: { faction_id: number; balance: number } }
	| { TraderQuotes: { quotes: TraderQuote[] } }
	| {
			CraftingCatalog: { items: ItemDefinition[]; recipes: RecipeDefinition[] };
	  }
	| {
			ProgressionUpdated: {
				entity_id: number;
				branches: ProgressionSnapshot[];
			};
	  }
	| { DynamicEventStarted: { event: DynamicEventSnapshot } }
	| { DynamicEventEnded: { event_id: number } }
	| { InteractableUpdated: { interactable: InteractableSnapshot } }
	| { RaidWarning: { raid: RaidStateSnapshot } }
	| { RaidStarted: { raid: RaidStateSnapshot } }
	| { RaidEnded: { raid: RaidStateSnapshot; attacker_won: boolean } }
	| { WorldMapChunk: { chunk_x: number; chunk_y: number; tiles: TileData[] } }
	| { Pong: { client_timestamp: number; server_timestamp: number } }
	| { ResourceDepleted: { entity_id: number } }
	| {
			XpGained: {
				branch: SkillBranch;
				amount: number;
				new_total: number;
				new_level: number;
			};
	  }
	| { CraftDenied: { reason: string } }
	| { TradeFailed: { reason: string } }
	| { AmmoChanged: { entity_id: number; ammo: number; max_ammo: number } }
	| { PowerState: { zone: ZoneId; networks: PowerNetworkSnapshot[] } }
	| { RaidTargets: { defender_faction_ids: number[] } }
	| { Disconnect: { reason: string } };
