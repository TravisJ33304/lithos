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
	private uiElements: Phaser.GameObjects.GameObject[] = [];

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
		this.load.image("heavy_flamethrower", "sprites/entities/heavy_flamethrower.png");
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
		this.load.image("projectile_bullet", "sprites/projectiles/projectile_bullet.png");
		this.load.image("projectile_artillery", "sprites/projectiles/projectile_artillery.png");
		this.load.image("projectile_laser", "sprites/projectiles/projectile_laser.png");
		this.load.image("mining_laser_beam", "sprites/projectiles/mining_laser_beam.png");

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
		const { width } = this.cameras.main;
		this.cameras.main.setBackgroundColor("#0d1117");

		this.add
			.text(width / 2, 60, "L I T H O S", {
				fontSize: "48px",
				color: "#58a6ff",
				fontFamily: "monospace",
				fontStyle: "bold",
			})
			.setOrigin(0.5);

		this.add
			.text(width / 2, 110, "SERVER BROWSER", {
				fontSize: "16px",
				color: "#8b949e",
				fontFamily: "monospace",
			})
			.setOrigin(0.5);

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
		const { width, height } = this.cameras.main;

		const loadingText = this.add
			.text(width / 2, height / 2, "Fetching servers...", {
				fontSize: "14px",
				color: "#8b949e",
				fontFamily: "monospace",
			})
			.setOrigin(0.5);

		try {
			this.serverList = await this.api.listServers();
			loadingText.destroy();
			this.renderServerList();
		} catch (_err) {
			loadingText.setText("Failed to fetch servers. Using localhost fallback.");
			loadingText.setColor("#ff4444");
			this.time.delayedCall(2000, () => {
				loadingText.destroy();
				this.showDirectConnect();
			});
		}
	}

	private renderServerList(): void {
		const { width, height } = this.cameras.main;

		if (this.serverList.length === 0) {
			const emptyText = this.add
				.text(width / 2, height / 2, "No servers available", {
					fontSize: "14px",
					color: "#8b949e",
					fontFamily: "monospace",
				})
				.setOrigin(0.5);
			this.uiElements.push(emptyText);
			gameUi.showMenu([]);
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

		let yOff = 160;
		for (const server of this.serverList) {
			const healthColor = server.healthy ? "#2ea043" : "#ff4444";
			const btn = this.add
				.text(
					width / 2,
					yOff,
					`${server.name}  [${server.region}]  ${server.population}/${server.capacity}`,
					{
						fontSize: "14px",
						color: "#c9d1d9",
						fontFamily: "monospace",
						backgroundColor: "#161b22",
						padding: { x: 12, y: 6 },
					},
				)
				.setOrigin(0.5)
				.setInteractive({ useHandCursor: true });

			const statusDot = this.add
				.circle(width / 2 - 140, yOff, 5, parseInt(healthColor.slice(1), 16))
				.setOrigin(0.5);

			btn.on("pointerover", () => btn.setColor("#58a6ff"));
			btn.on("pointerout", () => btn.setColor("#c9d1d9"));
			btn.on("pointerdown", () => {
				this.connectToServer(server.websocket_url);
			});

			this.uiElements.push(btn, statusDot);
			yOff += 40;
		}

		this.showDirectConnect(yOff + 20);
	}

	private showDirectConnect(yOffset?: number): void {
		const { width, height } = this.cameras.main;
		const y = yOffset ?? height / 2 + 100;

		const directBtn = this.add
			.text(width / 2, y, "[ Connect to localhost ]", {
				fontSize: "12px",
				color: "#666666",
				fontFamily: "monospace",
			})
			.setOrigin(0.5)
			.setInteractive({ useHandCursor: true });

		directBtn.on("pointerover", () => directBtn.setColor("#58a6ff"));
		directBtn.on("pointerout", () => directBtn.setColor("#666666"));
		directBtn.on("pointerdown", () => {
			this.connectToServer("ws://localhost:9001");
		});

		this.uiElements.push(directBtn);
	}

	private connectToServer(wsUrl: string): void {
		this.net.setEndpoint(wsUrl);
		this.scene.start("LoginScene", { net: this.net });
	}
}
