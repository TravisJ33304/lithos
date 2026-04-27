/**
 * AsteroidBaseScene — A faction's private asteroid base.
 *
 * Simple room for the MVP — demonstrates zone transitions.
 */

import * as Phaser from "phaser";
import type { NetworkClient } from "../net/NetworkClient";
import type { ServerMessage } from "../types/protocol";

interface BaseData {
	net: NetworkClient;
	entityId: number;
}

export class AsteroidBaseScene extends Phaser.Scene {
	private net!: NetworkClient;
	private myEntityId!: number;
	private spaceKey!: Phaser.Input.Keyboard.Key;

	constructor() {
		super({ key: "AsteroidBaseScene" });
	}

	init(data: BaseData): void {
		this.net = data.net;
		this.myEntityId = data.entityId;
	}

	create(): void {
		this.cameras.main.setBackgroundColor("#1a0a2e");

		const { width, height } = this.cameras.main;

		// Room walls.
		const graphics = this.add.graphics();
		graphics.lineStyle(2, 0x7c3aed, 0.8);
		graphics.strokeRect(width / 2 - 200, height / 2 - 150, 400, 300);

		// Title.
		this.add
			.text(width / 2, height / 2 - 180, "ASTEROID BASE", {
				fontSize: "24px",
				color: "#7c3aed",
				fontFamily: "monospace",
			})
			.setOrigin(0.5);

		// Player dot.
		this.add.circle(width / 2, height / 2, 14, 0x58a6ff);

		this.add
			.text(width / 2, height / 2 - 20, "YOU", {
				fontSize: "10px",
				color: "#58a6ff",
				fontFamily: "monospace",
			})
			.setOrigin(0.5);

		// Instructions.
		this.add
			.text(width / 2, height / 2 + 180, "[SPACE] Return to Overworld", {
				fontSize: "14px",
				color: "#666666",
				fontFamily: "monospace",
			})
			.setOrigin(0.5);

		// --- Input ---
		if (this.input.keyboard) {
			this.spaceKey = this.input.keyboard.addKey(
				Phaser.Input.Keyboard.KeyCodes.SPACE,
			);
		}

		// Network listener for zone changes.
		this.net.onMessage((msg: ServerMessage) => {
			if ("ZoneChanged" in msg) {
				if ("Overworld" in msg.ZoneChanged.zone) {
					this.scene.start("OverworldScene", {
						net: this.net,
						entityId: this.myEntityId,
					});
				}
			}
		});
	}

	update(): void {
		// Space to return to Overworld.
		if (this.spaceKey && Phaser.Input.Keyboard.JustDown(this.spaceKey)) {
			this.net.send({
				ZoneTransfer: { target: { Overworld: null } },
			});
		}
	}
}
