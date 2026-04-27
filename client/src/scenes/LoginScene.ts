import * as Phaser from "phaser";
import { NetworkClient } from "../net/NetworkClient";

export class LoginScene extends Phaser.Scene {
	private net!: NetworkClient;
	private bgGraphics!: Phaser.GameObjects.Graphics;

	constructor() {
		super({ key: "LoginScene" });
	}

	init(data: { net: NetworkClient }): void {
		this.net = data.net;
	}

	create(): void {
		this.cameras.main.setBackgroundColor("#0d1117");
		const { width, height } = this.cameras.main;

		// Dynamic animated background
		this.bgGraphics = this.add.graphics();
		this.bgGraphics.setDepth(0);

		// Title
		this.add
			.text(width / 2, height / 2 - 120, "L I T H O S", {
				fontSize: "48px",
				color: "#58a6ff",
				fontFamily: "monospace",
				fontStyle: "bold",
				shadow: {
					offsetX: 2,
					offsetY: 2,
					color: "#000000",
					blur: 4,
					fill: true,
				},
			})
			.setOrigin(0.5)
			.setDepth(10);

		this.add
			.text(width / 2, height / 2 - 70, "Multiplayer Survival Crafting", {
				fontSize: "14px",
				color: "#8b949e",
				fontFamily: "monospace",
			})
			.setOrigin(0.5)
			.setDepth(10);

		// Container for HTML input overlay
		const formHtml = `
			<div style="display: flex; flex-direction: column; gap: 10px; width: 250px; text-align: center; font-family: monospace; font-size: 14px;">
				<input type="text" id="username" placeholder="Username" style="padding: 10px; background: #161b22; border: 1px solid #30363d; color: #c9d1d9; border-radius: 4px; outline: none; font-family: monospace;" />
				<button id="loginBtn" style="padding: 10px; background: #238636; color: white; border: none; border-radius: 4px; cursor: pointer; font-family: monospace; font-weight: bold; margin-top: 10px;">ENTER GALAXY</button>
			</div>
		`;

		const domElement = this.add
			.dom(width / 2, height / 2 + 30)
			.createFromHTML(formHtml);
		domElement.setDepth(20);

		// Focus the input
		const inputEl = document.getElementById("username") as HTMLInputElement;
		if (inputEl) {
			inputEl.focus();
		}

		// Connect to the login button
		const btn = document.getElementById("loginBtn");
		if (btn) {
			btn.addEventListener("click", () => {
				const username = inputEl?.value.trim() || "guest";
				this.startConnection(username);
			});
		}

		// Also listen for Enter key
		if (inputEl) {
			inputEl.addEventListener("keypress", (e) => {
				if (e.key === "Enter") {
					const username = inputEl.value.trim() || "guest";
					this.startConnection(username);
				}
			});
		}

		// Handle JoinAck
		this.net.onMessage((msg) => {
			if ("JoinAck" in msg) {
				const ack = msg.JoinAck;
				this.scene.start("OverworldScene", {
					net: this.net,
					entityId: ack.entity_id,
					worldSeed: ack.world_seed,
				});
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

	private startConnection(username: string): void {
		// Change button text
		const btn = document.getElementById("loginBtn");
		if (btn) {
			btn.innerText = "CONNECTING...";
			btn.style.background = "#8b949e";
		}

		// Connect and send Join
		this.net
			.connect()
			.then(() => {
				this.net.send({ Join: { token: username } });
			})
			.catch((err) => {
				console.error("Connection failed", err);
				if (btn) {
					btn.innerText = "CONNECTION FAILED";
					btn.style.background = "#da3633";
				}
			});
	}
}
