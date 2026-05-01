import * as Phaser from "phaser";
import type { NetworkClient } from "../net/NetworkClient";
import { SupabaseAuthClient } from "../net/SupabaseAuthClient";
import { gameUi } from "../ui/GameUiManager";

export class LoginScene extends Phaser.Scene {
	private net!: NetworkClient;
	private auth!: SupabaseAuthClient;
	private username = "guest";
	private bgGraphics!: Phaser.GameObjects.Graphics;

	constructor() {
		super({ key: "LoginScene" });
	}

	init(data: { net: NetworkClient; username?: string }): void {
		this.net = data.net;
		this.username = data.username ?? "guest";
	}

	create(): void {
		gameUi.hideMenu();
		gameUi.showLogin(this.username, this.net.getEndpoint());
		this.auth = new SupabaseAuthClient();
		gameUi.onLoginRequested((payload) => this.startConnection(payload));
		this.cameras.main.setBackgroundColor("#0d1117");

		this.bgGraphics = this.add.graphics();
		this.bgGraphics.setDepth(0);

		this.net.onMessage((msg) => {
			if ("JoinAck" in msg) {
				const ack = msg.JoinAck;
				gameUi.hideLogin();
				gameUi.showGameplay();
				this.scene.start("OverworldScene", {
					net: this.net,
					entityId: ack.entity_id,
					worldSeed: ack.world_seed,
				});
			} else if ("CraftingCatalog" in msg) {
				gameUi.updateCraftingCatalog(
					msg.CraftingCatalog.items,
					msg.CraftingCatalog.recipes,
				);
			}
		});
	}

	update(time: number): void {
		// Animate background
		this.bgGraphics.clear();
		const { width, height } = this.cameras.main;

		for (let i = 0; i < 50; i++) {
			const x = (Math.sin(time * 0.0005 + i * 0.5) * 0.5 + 0.5) * width;
			const y = (Math.cos(time * 0.0003 + i * 0.8) * 0.5 + 0.5) * height;
			const radius = Math.sin(time * 0.002 + i) * 2 + 3;

			this.bgGraphics.fillStyle(0x30363d, 0.5);
			this.bgGraphics.fillCircle(x, y, radius);
		}
	}

	private async startConnection(payload: {
		username: string;
		password: string;
	}): Promise<void> {
		try {
			gameUi.setLoginStatus("Resolving auth token", "loading");
			const token = await this.resolveJoinToken(payload);
			gameUi.setLoginStatus("Connecting to server", "loading");
			await this.net.connect();
			gameUi.setLoginStatus("Joining shard", "loading");
			this.net.send({ Join: { token } });
			this.net.send("RequestCraftingState");
		} catch (err) {
			console.error("Connection failed", err);
			gameUi.setLoginStatus(
				err instanceof Error ? err.message : "Connection failed",
				"error",
			);
		}
	}

	private async resolveJoinToken(payload: {
		username: string;
		password: string;
	}): Promise<string> {
		if (this.auth.isConfigured() && payload.password.length > 0) {
			return this.auth.signInOrSignUp(payload.username, payload.password);
		}
		return payload.username;
	}
}
