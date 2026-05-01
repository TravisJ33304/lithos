import { decode } from "@msgpack/msgpack";
import type {
	ChatChannel,
	DynamicEventKind,
	DynamicEventSnapshot,
	EntitySnapshot,
	InteractableSnapshot,
	InventoryItemStack,
	InventorySnapshot,
	ItemDefinition,
	ItemRarity,
	PowerNetworkSnapshot,
	ProgressionSnapshot,
	RaidStateSnapshot,
	RecipeDefinition,
	ServerMessage,
	SkillBranch,
	SnapshotEntityType,
	TerrainType,
	TraderQuote,
	Vec2,
	ZoneId,
} from "../types/protocol";

function isRecord(x: unknown): x is Record<string, unknown> {
	return typeof x === "object" && x !== null && !Array.isArray(x);
}

function asNumber(x: unknown): number {
	if (typeof x === "number" && Number.isFinite(x)) return x;
	if (typeof x === "bigint") return Number(x);
	throw new Error(`expected number, got ${typeof x}`);
}

function asString(x: unknown): string {
	if (typeof x === "string") return x;
	throw new Error(`expected string, got ${typeof x}`);
}

function uuidFromBytes(b: unknown): string {
	if (!(b instanceof Uint8Array)) {
		throw new Error("expected Uint8Array for UUID");
	}
	if (b.byteLength !== 16) {
		throw new Error(`expected 16-byte UUID, got ${b.byteLength}`);
	}
	const h = [...b].map((x) => x.toString(16).padStart(2, "0")).join("");
	return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20, 32)}`;
}

function normalizeVec2(v: unknown): Vec2 {
	if (Array.isArray(v) && v.length >= 2) {
		return { x: asNumber(v[0]), y: asNumber(v[1]) };
	}
	if (isRecord(v) && "x" in v && "y" in v) {
		return { x: asNumber(v.x), y: asNumber(v.y) };
	}
	throw new Error("invalid Vec2");
}

export function normalizeZoneId(z: unknown): ZoneId {
	if (typeof z === "string" && z === "Overworld") {
		return { Overworld: null };
	}
	if (Array.isArray(z) && z.length === 1) {
		return normalizeZoneId(z[0]);
	}
	if (isRecord(z) && "Overworld" in z) {
		return { Overworld: null };
	}
	if (isRecord(z) && "AsteroidBase" in z) {
		return { AsteroidBase: asNumber(z.AsteroidBase) };
	}
	throw new Error(`invalid ZoneId: ${JSON.stringify(z)}`);
}

function normalizeChatChannel(c: unknown): ChatChannel {
	const s = asString(c);
	if (s === "Global" || s === "Faction") return s;
	throw new Error(`invalid ChatChannel: ${s}`);
}

function normalizeSnapshotEntityType(t: unknown): SnapshotEntityType {
	const s = asString(t);
	const allowed: SnapshotEntityType[] = [
		"Player",
		"Hostile",
		"Rover",
		"Drone",
		"AssaultWalker",
		"SniperWalker",
		"HeavyFlamethrower",
		"CoreWarden",
		"Trader",
		"ResourceNode",
		"Item",
		"Projectile",
		"Unknown",
	];
	if ((allowed as string[]).includes(s)) return s as SnapshotEntityType;
	throw new Error(`invalid SnapshotEntityType: ${s}`);
}

function normalizeSkillBranch(b: unknown): SkillBranch {
	const s = asString(b);
	const allowed: SkillBranch[] = [
		"Fabrication",
		"Extraction",
		"Ballistics",
		"Cybernetics",
	];
	if ((allowed as string[]).includes(s)) return s as SkillBranch;
	throw new Error(`invalid SkillBranch: ${s}`);
}

function normalizeDynamicEventKind(k: unknown): DynamicEventKind {
	const s = asString(k);
	const allowed: DynamicEventKind[] = [
		"MeteorShower",
		"SolarFlare",
		"CrashedFreighter",
	];
	if ((allowed as string[]).includes(s)) return s as DynamicEventKind;
	throw new Error(`invalid DynamicEventKind: ${s}`);
}

function normalizeTerrainType(v: unknown): TerrainType {
	const s = asString(v);
	const allowed: TerrainType[] = [
		"Empty",
		"Rock",
		"DeepRavine",
		"AsteroidField",
		"AutomataSpire",
	];
	if ((allowed as string[]).includes(s)) return s as TerrainType;
	throw new Error(`invalid TerrainType: ${s}`);
}

function normalizeEntitySnapshot(e: unknown): EntitySnapshot {
	if (Array.isArray(e) && e.length >= 5) {
		return {
			id: asNumber(e[0]),
			position: normalizeVec2(e[1]),
			velocity: normalizeVec2(e[2]),
			zone: normalizeZoneId(e[3]),
			entity_type: normalizeSnapshotEntityType(e[4]),
			subtype: e.length >= 6 ? asString(e[5]) : undefined,
		};
	}
	if (isRecord(e) && "id" in e) {
		return e as unknown as EntitySnapshot;
	}
	throw new Error("invalid EntitySnapshot");
}

function normalizeTraderQuote(q: unknown): TraderQuote {
	if (Array.isArray(q) && q.length >= 6) {
		return {
			trader_entity_id: asNumber(q[0]),
			item: asString(q[1]),
			buy_price: asNumber(q[2]),
			sell_price: asNumber(q[3]),
			demand_scalar: asNumber(q[4]),
			available_credits: asNumber(q[5]),
			daily_credit_limit: q.length >= 7 ? asNumber(q[6]) : 0,
			daily_credits_used: q.length >= 8 ? asNumber(q[7]) : 0,
		};
	}
	if (isRecord(q) && "trader_entity_id" in q) {
		return {
			...(q as unknown as TraderQuote),
			daily_credit_limit: asNumber(
				(q as Record<string, unknown>).daily_credit_limit ?? 0,
			),
			daily_credits_used: asNumber(
				(q as Record<string, unknown>).daily_credits_used ?? 0,
			),
		};
	}
	throw new Error("invalid TraderQuote");
}

function normalizeItemRarity(v: unknown): ItemRarity {
	const s = asString(v);
	if (s === "Common" || s === "Uncommon" || s === "Rare" || s === "Epic") {
		return s;
	}
	throw new Error(`invalid ItemRarity: ${s}`);
}

function normalizeInventoryItemStack(v: unknown): InventoryItemStack {
	if (isRecord(v)) {
		return {
			item: asString(v.item),
			quantity: asNumber(v.quantity),
			rarity: normalizeItemRarity(v.rarity),
			category: asString(v.category) as InventoryItemStack["category"],
		};
	}
	if (Array.isArray(v) && v.length >= 4) {
		return {
			item: asString(v[0]),
			quantity: asNumber(v[1]),
			rarity: normalizeItemRarity(v[2]),
			category: asString(v[3]) as InventoryItemStack["category"],
		};
	}
	throw new Error("invalid InventoryItemStack");
}

function normalizeInventorySnapshot(v: unknown): InventorySnapshot {
	if (isRecord(v)) {
		const items = Array.isArray(v.items)
			? v.items.map(normalizeInventoryItemStack)
			: [];
		return { entity_id: asNumber(v.entity_id), items };
	}
	if (Array.isArray(v) && v.length >= 2 && Array.isArray(v[1])) {
		return {
			entity_id: asNumber(v[0]),
			items: v[1].map(normalizeInventoryItemStack),
		};
	}
	throw new Error("invalid InventorySnapshot");
}

function normalizeItemDefinition(v: unknown): ItemDefinition {
	if (Array.isArray(v) && v.length >= 6) {
		return {
			item: asString(v[0]),
			display_name: asString(v[1]),
			description: asString(v[2]),
			rarity: normalizeItemRarity(v[3]),
			category: asString(v[4]) as ItemDefinition["category"],
			stack_limit: asNumber(v[5]),
		};
	}
	if (isRecord(v)) {
		return {
			item: asString(v.item),
			display_name: asString(v.display_name),
			description: asString(v.description),
			rarity: normalizeItemRarity(v.rarity),
			category: asString(v.category) as ItemDefinition["category"],
			stack_limit: asNumber(v.stack_limit),
		};
	}
	throw new Error("invalid ItemDefinition");
}

function normalizeRecipeDefinition(v: unknown): RecipeDefinition {
	if (Array.isArray(v) && v.length >= 5) {
		return {
			name: asString(v[0]),
			output: asString(v[1]),
			required_branch: normalizeSkillBranch(v[2]),
			required_level: asNumber(v[3]),
			inputs: Array.isArray(v[4]) ? v[4].map(asString) : [],
		};
	}
	if (isRecord(v)) {
		return {
			name: asString(v.name),
			output: asString(v.output),
			required_branch: normalizeSkillBranch(v.required_branch),
			required_level: asNumber(v.required_level),
			inputs: Array.isArray(v.inputs) ? v.inputs.map(asString) : [],
		};
	}
	throw new Error("invalid RecipeDefinition");
}

function normalizeInteractableSnapshot(v: unknown): InteractableSnapshot {
	if (isRecord(v)) {
		return {
			target_entity_id: asNumber(v.target_entity_id),
			kind: asString(v.kind) as InteractableSnapshot["kind"],
			required_tool:
				v.required_tool === null || v.required_tool === undefined
					? null
					: asString(v.required_tool),
			can_interact: Boolean(v.can_interact),
		};
	}
	throw new Error("invalid InteractableSnapshot");
}

function normalizePowerNetworkSnapshot(v: unknown): PowerNetworkSnapshot {
	if (Array.isArray(v) && v.length >= 6) {
		return {
			network_id: asNumber(v[0]),
			zone: normalizeZoneId(v[1]),
			generation_kw: asNumber(v[2]),
			load_kw: asNumber(v[3]),
			consumers_powered: asNumber(v[4]),
			consumers_total: asNumber(v[5]),
		};
	}
	if (isRecord(v)) {
		return {
			network_id: asNumber(v.network_id),
			zone: normalizeZoneId(v.zone),
			generation_kw: asNumber(v.generation_kw),
			load_kw: asNumber(v.load_kw),
			consumers_powered: asNumber(v.consumers_powered),
			consumers_total: asNumber(v.consumers_total),
		};
	}
	throw new Error("invalid PowerNetworkSnapshot");
}

function normalizeProgressionSnapshot(p: unknown): ProgressionSnapshot {
	if (Array.isArray(p) && p.length >= 4) {
		return {
			branch: normalizeSkillBranch(p[0]),
			level: asNumber(p[1]),
			xp: asNumber(p[2]),
			xp_to_next: asNumber(p[3]),
		};
	}
	if (isRecord(p) && "branch" in p) {
		return p as unknown as ProgressionSnapshot;
	}
	throw new Error("invalid ProgressionSnapshot");
}

function normalizeRaidState(r: unknown): RaidStateSnapshot {
	if (Array.isArray(r) && r.length >= 4) {
		return {
			attacker_faction_id: asNumber(r[0]),
			defender_faction_id: asNumber(r[1]),
			warning_remaining_seconds: asNumber(r[2]),
			breach_active: Boolean(r[3]),
		};
	}
	if (isRecord(r) && "attacker_faction_id" in r) {
		return r as unknown as RaidStateSnapshot;
	}
	throw new Error("invalid RaidStateSnapshot");
}

function normalizeDynamicEvent(ev: unknown): DynamicEventSnapshot {
	if (Array.isArray(ev) && ev.length >= 5) {
		return {
			event_id: asNumber(ev[0]),
			kind: normalizeDynamicEventKind(ev[1]),
			started_at_unix_ms: asNumber(ev[2]),
			expires_at_unix_ms: asNumber(ev[3]),
			description: asString(ev[4]),
		};
	}
	if (isRecord(ev) && "event_id" in ev) {
		return ev as unknown as DynamicEventSnapshot;
	}
	throw new Error("invalid DynamicEventSnapshot");
}

type JoinAckBody = Extract<ServerMessage, { JoinAck: unknown }>["JoinAck"];

function normalizeJoinAckPayloadFixed(payload: unknown): {
	JoinAck: JoinAckBody;
} {
	if (Array.isArray(payload) && payload.length >= 4) {
		return {
			JoinAck: {
				player_id: uuidFromBytes(payload[0]),
				entity_id: asNumber(payload[1]),
				zone: normalizeZoneId(payload[2]),
				world_seed: asNumber(payload[3]),
			},
		};
	}
	if (isRecord(payload) && "player_id" in payload) {
		return { JoinAck: payload as unknown as JoinAckBody };
	}
	throw new Error("invalid JoinAck payload");
}

function normalizeStateSnapshotPayload(payload: unknown): ServerMessage {
	if (Array.isArray(payload) && payload.length >= 3) {
		const entitiesRaw = payload[2];
		if (!Array.isArray(entitiesRaw)) throw new Error("StateSnapshot entities");
		return {
			StateSnapshot: {
				tick: asNumber(payload[0]),
				last_processed_seq: asNumber(payload[1]),
				entities: entitiesRaw.map(normalizeEntitySnapshot),
			},
		};
	}
	if (isRecord(payload) && "tick" in payload) {
		return {
			StateSnapshot: payload as unknown as Extract<
				ServerMessage,
				{ StateSnapshot: unknown }
			>["StateSnapshot"],
		};
	}
	throw new Error("invalid StateSnapshot payload");
}

function singleStringArray(payload: unknown): string {
	if (!Array.isArray(payload) || payload.length !== 1) {
		throw new Error("expected single-element string array");
	}
	return asString(payload[0]);
}

export function normalizeServerMessage(raw: unknown): ServerMessage {
	if (!isRecord(raw)) {
		throw new Error("invalid server message root");
	}
	const keys = Object.keys(raw);
	if (keys.length !== 1) {
		throw new Error("expected exactly one variant key");
	}
	const variant = keys[0];
	const payload = raw[variant];

	switch (variant) {
		case "JoinAck":
			return normalizeJoinAckPayloadFixed(payload);
		case "StateSnapshot":
			return normalizeStateSnapshotPayload(payload);
		case "ZoneChanged": {
			if (isRecord(payload) && "zone" in payload) {
				return {
					ZoneChanged: {
						zone: normalizeZoneId((payload as { zone: unknown }).zone),
					},
				};
			}
			return { ZoneChanged: { zone: normalizeZoneId(payload) } };
		}
		case "HealthChanged": {
			if (Array.isArray(payload) && payload.length >= 3) {
				return {
					HealthChanged: {
						entity_id: asNumber(payload[0]),
						health: asNumber(payload[1]),
						max_health: asNumber(payload[2]),
					},
				};
			}
			return {
				HealthChanged: payload as unknown as Extract<
					ServerMessage,
					{ HealthChanged: unknown }
				>["HealthChanged"],
			};
		}
		case "OxygenChanged": {
			if (Array.isArray(payload) && payload.length >= 3) {
				return {
					OxygenChanged: {
						entity_id: asNumber(payload[0]),
						current: asNumber(payload[1]),
						max: asNumber(payload[2]),
					},
				};
			}
			return {
				OxygenChanged: payload as unknown as Extract<
					ServerMessage,
					{ OxygenChanged: unknown }
				>["OxygenChanged"],
			};
		}
		case "PlayerDied": {
			if (Array.isArray(payload) && payload.length >= 1) {
				return { PlayerDied: { entity_id: asNumber(payload[0]) } };
			}
			return {
				PlayerDied: payload as unknown as Extract<
					ServerMessage,
					{ PlayerDied: unknown }
				>["PlayerDied"],
			};
		}
		case "InventoryUpdated": {
			if (Array.isArray(payload) && payload.length >= 2) {
				return {
					InventoryUpdated: {
						entity_id: asNumber(payload[0]),
						items_json: asString(payload[1]),
					},
				};
			}
			return {
				InventoryUpdated: payload as unknown as Extract<
					ServerMessage,
					{ InventoryUpdated: unknown }
				>["InventoryUpdated"],
			};
		}
		case "InventorySnapshot": {
			if (isRecord(payload) && "inventory" in payload) {
				return {
					InventorySnapshot: {
						inventory: normalizeInventorySnapshot(payload.inventory),
					},
				};
			}
			return {
				InventorySnapshot: {
					inventory: normalizeInventorySnapshot(payload),
				},
			};
		}
		case "SpawnProjectile": {
			if (Array.isArray(payload) && payload.length >= 3) {
				return {
					SpawnProjectile: {
						entity_id: asNumber(payload[0]),
						position: normalizeVec2(payload[1]),
						velocity: normalizeVec2(payload[2]),
					},
				};
			}
			return {
				SpawnProjectile: payload as unknown as Extract<
					ServerMessage,
					{ SpawnProjectile: unknown }
				>["SpawnProjectile"],
			};
		}
		case "ChatMessage": {
			if (Array.isArray(payload) && payload.length >= 4) {
				return {
					ChatMessage: {
						from_entity_id: asNumber(payload[0]),
						channel: normalizeChatChannel(payload[1]),
						text: asString(payload[2]),
						sent_at_unix_ms: asNumber(payload[3]),
					},
				};
			}
			return {
				ChatMessage: payload as unknown as Extract<
					ServerMessage,
					{ ChatMessage: unknown }
				>["ChatMessage"],
			};
		}
		case "CreditsChanged": {
			if (Array.isArray(payload) && payload.length >= 2) {
				return {
					CreditsChanged: {
						faction_id: asNumber(payload[0]),
						balance: asNumber(payload[1]),
					},
				};
			}
			return {
				CreditsChanged: payload as unknown as Extract<
					ServerMessage,
					{ CreditsChanged: unknown }
				>["CreditsChanged"],
			};
		}
		case "TraderQuotes": {
			if (
				Array.isArray(payload) &&
				payload.length >= 1 &&
				Array.isArray(payload[0])
			) {
				const quotes = (payload[0] as unknown[]).map(normalizeTraderQuote);
				return { TraderQuotes: { quotes } };
			}
			return {
				TraderQuotes: payload as unknown as Extract<
					ServerMessage,
					{ TraderQuotes: unknown }
				>["TraderQuotes"],
			};
		}
		case "ProgressionUpdated": {
			if (
				Array.isArray(payload) &&
				payload.length >= 2 &&
				Array.isArray(payload[1])
			) {
				const branches = (payload[1] as unknown[]).map(
					normalizeProgressionSnapshot,
				);
				return {
					ProgressionUpdated: {
						entity_id: asNumber(payload[0]),
						branches,
					},
				};
			}
			return {
				ProgressionUpdated: payload as unknown as Extract<
					ServerMessage,
					{ ProgressionUpdated: unknown }
				>["ProgressionUpdated"],
			};
		}
		case "CraftingCatalog": {
			if (Array.isArray(payload) && payload.length >= 2) {
				return {
					CraftingCatalog: {
						items: Array.isArray(payload[0])
							? payload[0].map(normalizeItemDefinition)
							: [],
						recipes: Array.isArray(payload[1])
							? payload[1].map(normalizeRecipeDefinition)
							: [],
					},
				};
			}
			if (isRecord(payload)) {
				return {
					CraftingCatalog: {
						items: Array.isArray(payload.items)
							? payload.items.map(normalizeItemDefinition)
							: [],
						recipes: Array.isArray(payload.recipes)
							? payload.recipes.map(normalizeRecipeDefinition)
							: [],
					},
				};
			}
			throw new Error("invalid CraftingCatalog");
		}
		case "DynamicEventStarted": {
			if (Array.isArray(payload) && payload.length >= 1) {
				return {
					DynamicEventStarted: { event: normalizeDynamicEvent(payload[0]) },
				};
			}
			return {
				DynamicEventStarted: payload as unknown as Extract<
					ServerMessage,
					{ DynamicEventStarted: unknown }
				>["DynamicEventStarted"],
			};
		}
		case "DynamicEventEnded": {
			if (Array.isArray(payload) && payload.length >= 1) {
				return { DynamicEventEnded: { event_id: asNumber(payload[0]) } };
			}
			return {
				DynamicEventEnded: payload as unknown as Extract<
					ServerMessage,
					{ DynamicEventEnded: unknown }
				>["DynamicEventEnded"],
			};
		}
		case "InteractableUpdated": {
			if (isRecord(payload) && "interactable" in payload) {
				return {
					InteractableUpdated: {
						interactable: normalizeInteractableSnapshot(payload.interactable),
					},
				};
			}
			return {
				InteractableUpdated: {
					interactable: normalizeInteractableSnapshot(payload),
				},
			};
		}
		case "RaidWarning": {
			if (Array.isArray(payload) && payload.length >= 1) {
				return { RaidWarning: { raid: normalizeRaidState(payload[0]) } };
			}
			return {
				RaidWarning: payload as unknown as Extract<
					ServerMessage,
					{ RaidWarning: unknown }
				>["RaidWarning"],
			};
		}
		case "RaidStarted": {
			if (Array.isArray(payload) && payload.length >= 1) {
				return { RaidStarted: { raid: normalizeRaidState(payload[0]) } };
			}
			return {
				RaidStarted: payload as unknown as Extract<
					ServerMessage,
					{ RaidStarted: unknown }
				>["RaidStarted"],
			};
		}
		case "RaidEnded": {
			if (Array.isArray(payload) && payload.length >= 2) {
				return {
					RaidEnded: {
						raid: normalizeRaidState(payload[0]),
						attacker_won: Boolean(payload[1]),
					},
				};
			}
			return {
				RaidEnded: payload as unknown as Extract<
					ServerMessage,
					{ RaidEnded: unknown }
				>["RaidEnded"],
			};
		}
		case "Pong": {
			if (Array.isArray(payload) && payload.length >= 2) {
				return {
					Pong: {
						client_timestamp: asNumber(payload[0]),
						server_timestamp: asNumber(payload[1]),
					},
				};
			}
			return {
				Pong: payload as unknown as Extract<
					ServerMessage,
					{ Pong: unknown }
				>["Pong"],
			};
		}
		case "ResourceDepleted": {
			if (Array.isArray(payload) && payload.length >= 1) {
				return { ResourceDepleted: { entity_id: asNumber(payload[0]) } };
			}
			return {
				ResourceDepleted: payload as unknown as Extract<
					ServerMessage,
					{ ResourceDepleted: unknown }
				>["ResourceDepleted"],
			};
		}
		case "XpGained": {
			if (Array.isArray(payload) && payload.length >= 4) {
				return {
					XpGained: {
						branch: normalizeSkillBranch(payload[0]),
						amount: asNumber(payload[1]),
						new_total: asNumber(payload[2]),
						new_level: asNumber(payload[3]),
					},
				};
			}
			return {
				XpGained: payload as unknown as Extract<
					ServerMessage,
					{ XpGained: unknown }
				>["XpGained"],
			};
		}
		case "CraftDenied": {
			if (Array.isArray(payload)) {
				return { CraftDenied: { reason: singleStringArray(payload) } };
			}
			return {
				CraftDenied: payload as unknown as Extract<
					ServerMessage,
					{ CraftDenied: unknown }
				>["CraftDenied"],
			};
		}
		case "TradeFailed": {
			if (Array.isArray(payload)) {
				return { TradeFailed: { reason: singleStringArray(payload) } };
			}
			return {
				TradeFailed: payload as unknown as Extract<
					ServerMessage,
					{ TradeFailed: unknown }
				>["TradeFailed"],
			};
		}
		case "AmmoChanged": {
			if (Array.isArray(payload) && payload.length >= 3) {
				return {
					AmmoChanged: {
						entity_id: asNumber(payload[0]),
						ammo: asNumber(payload[1]),
						max_ammo: asNumber(payload[2]),
					},
				};
			}
			return {
				AmmoChanged: payload as unknown as Extract<
					ServerMessage,
					{ AmmoChanged: unknown }
				>["AmmoChanged"],
			};
		}
		case "PowerState": {
			if (Array.isArray(payload) && payload.length >= 2) {
				return {
					PowerState: {
						zone: normalizeZoneId(payload[0]),
						networks: Array.isArray(payload[1])
							? payload[1].map(normalizePowerNetworkSnapshot)
							: [],
					},
				};
			}
			if (isRecord(payload)) {
				return {
					PowerState: {
						zone: normalizeZoneId(payload.zone),
						networks: Array.isArray(payload.networks)
							? payload.networks.map(normalizePowerNetworkSnapshot)
							: [],
					},
				};
			}
			throw new Error("invalid PowerState");
		}
		case "RaidTargets": {
			if (isRecord(payload)) {
				return {
					RaidTargets: {
						defender_faction_ids: Array.isArray(payload.defender_faction_ids)
							? payload.defender_faction_ids.map(asNumber)
							: [],
					},
				};
			}
			throw new Error("invalid RaidTargets");
		}
		case "WorldMapChunk": {
			if (Array.isArray(payload) && payload.length >= 3) {
				const tilesRaw = payload[2];
				if (!Array.isArray(tilesRaw)) throw new Error("WorldMapChunk tiles");
				const tiles = tilesRaw.map((t: unknown) => {
					if (Array.isArray(t) && t.length >= 3) {
						return {
							terrain: normalizeTerrainType(t[0]),
							ceiling: asString(t[1]) as "Open" | "Enclosed",
							height: asNumber(t[2]),
						};
					}
					if (isRecord(t)) {
						return {
							terrain: normalizeTerrainType(t.terrain),
							ceiling: asString(t.ceiling) as "Open" | "Enclosed",
							height: asNumber(t.height),
						};
					}
					throw new Error("invalid TileData");
				});
				return {
					WorldMapChunk: {
						chunk_x: asNumber(payload[0]),
						chunk_y: asNumber(payload[1]),
						tiles,
					},
				};
			}
			if (isRecord(payload) && "chunk_x" in payload) {
				return {
					WorldMapChunk: payload as unknown as Extract<
						ServerMessage,
						{ WorldMapChunk: unknown }
					>["WorldMapChunk"],
				};
			}
			throw new Error("invalid WorldMapChunk payload");
		}
		case "Disconnect": {
			if (Array.isArray(payload)) {
				return { Disconnect: { reason: singleStringArray(payload) } };
			}
			return {
				Disconnect: payload as unknown as Extract<
					ServerMessage,
					{ Disconnect: unknown }
				>["Disconnect"],
			};
		}
		default:
			throw new Error(`unknown server variant: ${variant}`);
	}
}

export function decodeServerMessage(bytes: Uint8Array): ServerMessage {
	const raw = decode(bytes) as unknown;
	return normalizeServerMessage(raw);
}
