/**
 * BootScene — Connects to the game server, then transitions to Overworld.
 */

import * as Phaser from "phaser";
import { NetworkClient } from "../net/NetworkClient";

// Server WebSocket URL (configurable via env or fallback to localhost).
const WS_URL = "ws://localhost:9001";

export class BootScene extends Phaser.Scene {
	private net!: NetworkClient;

	constructor() {
		super({ key: "BootScene" });
	}

	create(): void {
		const { width, height } = this.cameras.main;

		const title = this.add
			.text(width / 2, height / 2 - 30, "LITHOS", {
				fontSize: "48px",
				color: "#ffffff",
				fontFamily: "monospace",
			})
			.setOrigin(0.5);

		const status = this.add
			.text(width / 2, height / 2 + 30, "Connecting...", {
				fontSize: "16px",
				color: "#888888",
				fontFamily: "monospace",
			})
			.setOrigin(0.5);

		// Pulse animation on the title.
		this.tweens.add({
			targets: title,
			alpha: { from: 1, to: 0.5 },
			duration: 800,
			yoyo: true,
			repeat: -1,
		});

		this.net = new NetworkClient();

		// For MVP, proceed directly to login with network client configured.
		this.net.setEndpoint(WS_URL);
		status.setText(`Server: ${WS_URL}`);

		this.time.delayedCall(500, () => {
			this.scene.start("LoginScene", { net: this.net });
		});
	}
}
