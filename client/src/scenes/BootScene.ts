/**
 * BootScene — Fetches server list from Central API, shows server browser,
 * then transitions to LoginScene with selected server.
 */

import * as Phaser from "phaser";
import { ApiClient } from "../net/ApiClient";
import { NetworkClient } from "../net/NetworkClient";
import type { ServerListing } from "../types/protocol";
import { gameUi } from "../ui/GameUiManager";

// Central API URL (configurable via env or fallback to localhost).
const API_URL = "http://localhost:3000";

export class BootScene extends Phaser.Scene {
	private api!: ApiClient;
	private net!: NetworkClient;
	private serverList: ServerListing[] = [];

	constructor() {
		super({ key: "BootScene" });
	}

	preload(): void {
		// Entity sprites
		this.load.image("player", "sprites/entities/player.png");
		this.load.image("rover", "sprites/entities/rover.png");
		this.load.image("drone", "sprites/entities/drone.png");
		this.load.image("assault_walker", "sprites/entities/assault_walker.png");
		this.load.image("sniper_walker", "sprites/entities/sniper_walker.png");
		this.load.image(
			"heavy_flamethrower",
			"sprites/entities/heavy_flamethrower.png",
		);
		this.load.image("siege_unit", "sprites/entities/siege_unit.png");
		this.load.image("core_warden", "sprites/entities/core_warden.png");
		this.load.image("trader", "sprites/entities/trader.png");
		this.load.image("item_drop", "sprites/entities/item_drop.png");

		// Resource sprites
		this.load.image("node_iron", "sprites/resources/node_iron.png");
		this.load.image("node_copper", "sprites/resources/node_copper.png");
		this.load.image("node_silica", "sprites/resources/node_silica.png");
		this.load.image("node_uranium", "sprites/resources/node_uranium.png");
		this.load.image("node_plutonium", "sprites/resources/node_plutonium.png");
		this.load.image("node_biomass", "sprites/resources/node_biomass.png");

		// Structure sprites
		this.load.image("wall_segment", "sprites/structures/wall_segment.png");
		this.load.image("door", "sprites/structures/door.png");
		this.load.image("generator", "sprites/structures/generator.png");
		this.load.image("workbench", "sprites/structures/workbench.png");

		// Projectile sprites
		this.load.image(
			"projectile_bullet",
			"sprites/projectiles/projectile_bullet.png",
		);
		this.load.image(
			"projectile_artillery",
			"sprites/projectiles/projectile_artillery.png",
		);
		this.load.image(
			"projectile_laser",
			"sprites/projectiles/projectile_laser.png",
		);
		this.load.image(
			"mining_laser_beam",
			"sprites/projectiles/mining_laser_beam.png",
		);

		// Particle textures
		this.load.image("fx_muzzle_flash", "sprites/particles/fx_muzzle_flash.png");
		this.load.image("fx_explosion", "sprites/particles/fx_explosion.png");
		this.load.image("fx_spark", "sprites/particles/fx_spark.png");
		this.load.image("fx_fire_dot", "sprites/particles/fx_fire_dot.png");
		this.load.image("fx_smoke_puff", "sprites/particles/fx_smoke_puff.png");
		this.load.image("fx_warp_ring", "sprites/particles/fx_warp_ring.png");
		this.load.image("fx_hit_spark", "sprites/particles/fx_hit_spark.png");
		this.load.image("fx_mining_spark", "sprites/particles/fx_mining_spark.png");
	}

	create(): void {
		this.cameras.main.setBackgroundColor("#0d1117");
		this.api = new ApiClient(API_URL);
		this.net = new NetworkClient();
		gameUi.hideAllGameplay();
		gameUi.onJoinRequested(({ username, endpoint }) => {
			this.net.setEndpoint(endpoint);
			this.scene.start("LoginScene", { net: this.net, username });
		});

		this.fetchServers();
	}

	private async fetchServers(): Promise<void> {
		gameUi.showLoading("Fetching servers");
		try {
			this.serverList = await this.api.listServers();
			gameUi.hideLoading();
			this.renderServerList();
		} catch (_err) {
			gameUi.hideLoading();
			this.showDirectConnect();
		}
	}

	private renderServerList(): void {
		if (this.serverList.length === 0) {
			this.showDirectConnect();
			return;
		}
		gameUi.showMenu(
			this.serverList.map((server) => ({
				name: server.name,
				endpoint: server.websocket_url,
				detail: `[${server.region}] ${server.population}/${server.capacity}`,
			})),
		);
	}

	private showDirectConnect(): void {
		gameUi.showMenu([
			{
				name: "Local Dev Shard",
				endpoint: "ws://localhost:9001",
				detail: "[local] direct connect",
			},
		]);
	}
}
