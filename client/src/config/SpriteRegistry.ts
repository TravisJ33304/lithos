// client/src/config/SpriteRegistry.ts
import type { SnapshotEntityType } from "../types/protocol";

export interface SpriteDef {
	texture: string;
	scale?: number;
	tint?: number;
}

export const SPRITE_REGISTRY: Record<string, SpriteDef> = {
	// Entities
	Player: { texture: "player", scale: 1.0 },
	Hostile: { texture: "rover", scale: 1.0 },
	Rover: { texture: "rover", scale: 1.0 },
	Drone: { texture: "drone", scale: 1.0 },
	AssaultWalker: { texture: "assault_walker", scale: 1.0 },
	SniperWalker: { texture: "sniper_walker", scale: 1.0 },
	HeavyFlamethrower: { texture: "heavy_flamethrower", scale: 1.0 },
	CoreWarden: { texture: "core_warden", scale: 1.0 },
	Trader: { texture: "trader", scale: 1.0 },

	// Resources
	ResourceNode: { texture: "node_iron", scale: 1.0 },

	// Items / Projectiles
	Item: { texture: "item_drop", scale: 1.0 },
	Projectile: { texture: "projectile_bullet", scale: 1.0 },

	// Structures
	Unknown: { texture: "wall_segment", scale: 1.0 },
};

/** Resolve which texture to use for a given snapshot entity. */
export function resolveSprite(type: SnapshotEntityType): SpriteDef {
	return SPRITE_REGISTRY[type] ?? { texture: "player", scale: 1.0 };
}

/** Override resource node texture based on server subtype (future). */
export function resolveResourceSprite(subtype: string): SpriteDef {
	const map: Record<string, string> = {
		iron: "node_iron",
		copper: "node_copper",
		silica: "node_silica",
		uranium: "node_uranium",
		plutonium: "node_plutonium",
		biomass: "node_biomass",
	};
	return { texture: map[subtype] ?? "node_iron", scale: 1.0 };
}
