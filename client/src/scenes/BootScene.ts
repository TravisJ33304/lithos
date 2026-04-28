/**
 * BootScene — Fetches server list from Central API, shows server browser,
 * then transitions to LoginScene with selected server.
 */

import * as Phaser from "phaser";
import { ApiClient } from "../net/ApiClient";
import { NetworkClient } from "../net/NetworkClient";
import type { ServerListing } from "../types/protocol";

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
			this.showDirectConnect();
			return;
		}

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
