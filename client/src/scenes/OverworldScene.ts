/**
 * OverworldScene — The main shared game world.
 *
 * Handles player movement, entity rendering, interpolation, and zone transitions.
 */

import * as Phaser from "phaser";
import { createNoise2D } from "simplex-noise";
import type { NetworkClient } from "../net/NetworkClient";
import type {
	EntitySnapshot,
	ServerMessage,
	TileData,
	Vec2,
} from "../types/protocol";

/** A single chunk of the tilemap received from the server. */
interface TileChunk {
	tiles: TileData[];
	graphics: Phaser.GameObjects.Graphics[];
}

/** Data passed from BootScene. */
interface OverworldData {
	net: NetworkClient;
	playerId: string;
	entityId: number;
	zone: { Overworld: null } | { AsteroidBase: number };
	worldSeed?: number;
}

/** Runtime state for a rendered entity. */
interface RenderedEntity {
	sprite: Phaser.GameObjects.Shape;
	facingLine?: Phaser.GameObjects.Graphics;
	label: Phaser.GameObjects.Text;
	targetX: number;
	targetY: number;
}

const WORLD_SIZE = 4000;
const INTERPOLATION_SPEED = 0.2;

export class OverworldScene extends Phaser.Scene {
	private net!: NetworkClient;
	private myEntityId!: number;
	private entities: Map<number, RenderedEntity> = new Map();
	private projectileIds: Set<number> = new Set();
	private chunks: Map<string, TileChunk> = new Map();
	private chunkGraphicsPool: Phaser.GameObjects.Graphics[] = [];
	private chunkSize = 32;
	private tileSize = 40;
	private healthText!: Phaser.GameObjects.Text;
	private inventoryText!: Phaser.GameObjects.Text;
	private crosshair!: Phaser.GameObjects.Graphics;
	private inventoryItems: string[] = [];
	private currentHealth: number = 100;
	private maxHealth: number = 100;
	private isDead: boolean = false;
	private cursors!: Phaser.Types.Input.Keyboard.CursorKeys;
	private wasd!: {
		W: Phaser.Input.Keyboard.Key;
		A: Phaser.Input.Keyboard.Key;
		S: Phaser.Input.Keyboard.Key;
		D: Phaser.Input.Keyboard.Key;
	};
	private spaceKey!: Phaser.Input.Keyboard.Key;
	private inputSeq = 0;
	private fpsText!: Phaser.GameObjects.Text;
	private tickText!: Phaser.GameObjects.Text;
	private lastDirection: Vec2 = { x: 0, y: 0 };
	private noise2D!: (x: number, y: number) => number;
	private worldSeed: number = 12345;
	private minimap!: Phaser.Cameras.Scene2D.Camera;
	private craftKey!: Phaser.Input.Keyboard.Key;
	private craftPanelVisible = false;
	private craftPanelElements: Phaser.GameObjects.GameObject[] = [];

	private buildKey!: Phaser.Input.Keyboard.Key;
	private buildMode = false;
	private buildGhost!: Phaser.GameObjects.Rectangle;
	private selectedStructure = "wall_segment";

	// Hotbar state
	private hotbarSlot = 0; // 0 = unarmed/fire, 1-9 = inventory items
	private hotbarElements: Phaser.GameObjects.GameObject[] = [];
	private xpText!: Phaser.GameObjects.Text;
	private xpFlashText!: Phaser.GameObjects.Text;
	private craftDeniedText!: Phaser.GameObjects.Text;

	// Economy state
	private traderQuotes: import("../types/protocol").TraderQuote[] = [];
	private factionCredits = 0;
	private creditsText!: Phaser.GameObjects.Text;
	private tradeFailedText!: Phaser.GameObjects.Text;

	// Life support state
	private currentOxygen = 100;
	private maxOxygen = 100;
	private oxygenText!: Phaser.GameObjects.Text;

	// Weapon state
	private currentAmmo = 0;
	private maxAmmo = 0;
	private ammoText!: Phaser.GameObjects.Text;

	// Chat state
	private chatMessages: Array<{ from: string; text: string; color: string }> =
		[];
	private chatVisible = false;
	private chatElements: Phaser.GameObjects.GameObject[] = [];
	private chatInput = "";

	constructor() {
		super({ key: "OverworldScene" });
	}

	init(data: OverworldData): void {
		this.net = data.net;
		this.myEntityId = data.entityId;
		if (data.worldSeed) {
			this.worldSeed = data.worldSeed;
		}

		// Create a simple deterministic random function for simplex-noise
		let s = this.worldSeed;
		const random = () => {
			const x = Math.sin(s++) * 10000;
			return x - Math.floor(x);
		};
		this.noise2D = createNoise2D(random);
	}

	create(): void {
		// --- World background ---
		this.cameras.main.setBackgroundColor("#0d1117");

		// Grid lines for spatial reference.
		const graphics = this.add.graphics();
		graphics.lineStyle(1, 0x1a2332, 0.5);
		const gridStep = 200;
		for (let x = -WORLD_SIZE / 2; x <= WORLD_SIZE / 2; x += gridStep) {
			graphics.lineBetween(x, -WORLD_SIZE / 2, x, WORLD_SIZE / 2);
		}
		for (let y = -WORLD_SIZE / 2; y <= WORLD_SIZE / 2; y += gridStep) {
			graphics.lineBetween(-WORLD_SIZE / 2, y, WORLD_SIZE / 2, y);
		}

		// World bounds border.
		graphics.lineStyle(2, 0x30a14e, 0.8);
		graphics.strokeRect(
			-WORLD_SIZE / 2,
			-WORLD_SIZE / 2,
			WORLD_SIZE,
			WORLD_SIZE,
		);

		// Camera follows player (set up after first snapshot).
		this.cameras.main.setBounds(
			-WORLD_SIZE / 2 - 100,
			-WORLD_SIZE / 2 - 100,
			WORLD_SIZE + 200,
			WORLD_SIZE + 200,
		);

		// --- Input ---
		if (this.input.keyboard) {
			this.cursors = this.input.keyboard.createCursorKeys();
			this.wasd = {
				W: this.input.keyboard.addKey(Phaser.Input.Keyboard.KeyCodes.W),
				A: this.input.keyboard.addKey(Phaser.Input.Keyboard.KeyCodes.A),
				S: this.input.keyboard.addKey(Phaser.Input.Keyboard.KeyCodes.S),
				D: this.input.keyboard.addKey(Phaser.Input.Keyboard.KeyCodes.D),
			};
			this.spaceKey = this.input.keyboard.addKey(
				Phaser.Input.Keyboard.KeyCodes.SPACE,
			);
		}

		// --- Minimap ---
		const cw = this.cameras.main.width;
		this.minimap = this.cameras
			.add(cw - 160, 10, 150, 150)
			.setZoom(0.05)
			.setName("minimap");
		this.minimap.setBackgroundColor(0x000000);

		// --- HUD ---
		this.fpsText = this.add
			.text(10, 10, "FPS: 0", {
				fontSize: "14px",
				color: "#30a14e",
				fontFamily: "monospace",
			})
			.setScrollFactor(0)
			.setDepth(100);

		this.tickText = this.add
			.text(10, 28, "Tick: 0", {
				fontSize: "14px",
				color: "#30a14e",
				fontFamily: "monospace",
			})
			.setScrollFactor(0)
			.setDepth(100);

		// Zone transfer instruction.
		this.add
			.text(10, 46, "[SPACE] Warp to Asteroid Base | [CLICK] Shoot", {
				fontSize: "12px",
				color: "#666666",
				fontFamily: "monospace",
			})
			.setScrollFactor(0)
			.setDepth(100);

		// Crafting instructions
		this.add
			.text(10, 82, "[C] Crafting Panel\n[B] Toggle Build Mode", {
				fontSize: "12px",
				color: "#666666",
				fontFamily: "monospace",
			})
			.setScrollFactor(0)
			.setDepth(100);

		// C key for crafting, B for building
		if (this.input.keyboard) {
			this.craftKey = this.input.keyboard.addKey(
				Phaser.Input.Keyboard.KeyCodes.C,
			);
			this.buildKey = this.input.keyboard.addKey(
				Phaser.Input.Keyboard.KeyCodes.B,
			);
			// Hotbar number keys 1-9
			for (let i = 1; i <= 9; i++) {
				this.input.keyboard
					.addKey(
						Phaser.Input.Keyboard.KeyCodes[
							`${i}` as keyof typeof Phaser.Input.Keyboard.KeyCodes
						],
					)
					.on("down", () => {
						this.hotbarSlot = i;
						this.updateHotbarUI();
					});
			}
			// 0 key for unarmed/fire mode
			this.input.keyboard
				.addKey(Phaser.Input.Keyboard.KeyCodes.ZERO)
				.on("down", () => {
					this.hotbarSlot = 0;
					this.updateHotbarUI();
				});

			// Chat typing input.
			this.input.keyboard.on("keydown", (event: KeyboardEvent) => {
				if (!this.chatVisible) return;
				if (event.key === "Enter") return; // handled by toggleChat
				if (event.key === "Backspace") {
					this.chatInput = this.chatInput.slice(0, -1);
				} else if (event.key.length === 1 && this.chatInput.length < 100) {
					this.chatInput += event.key;
				}
				this.renderChatUI();
			});
		}

		// XP text
		this.xpText = this.add
			.text(10, 100, "XP: --", {
				fontSize: "12px",
				color: "#30a14e",
				fontFamily: "monospace",
			})
			.setScrollFactor(0)
			.setDepth(100);

		this.xpFlashText = this.add
			.text(
				this.cameras.main.width / 2,
				this.cameras.main.height / 2 - 50,
				"",
				{
					fontSize: "16px",
					color: "#2ea043",
					fontFamily: "monospace",
					fontStyle: "bold",
				},
			)
			.setScrollFactor(0)
			.setDepth(200)
			.setOrigin(0.5);

		this.craftDeniedText = this.add
			.text(
				this.cameras.main.width / 2,
				this.cameras.main.height / 2 + 50,
				"",
				{
					fontSize: "14px",
					color: "#ff4444",
					fontFamily: "monospace",
					fontStyle: "bold",
				},
			)
			.setScrollFactor(0)
			.setDepth(200)
			.setOrigin(0.5);

		this.creditsText = this.add
			.text(10, 116, "Credits: 0", {
				fontSize: "12px",
				color: "#f0a000",
				fontFamily: "monospace",
			})
			.setScrollFactor(0)
			.setDepth(100);

		this.tradeFailedText = this.add
			.text(
				this.cameras.main.width / 2,
				this.cameras.main.height / 2 + 80,
				"",
				{
					fontSize: "14px",
					color: "#ff4444",
					fontFamily: "monospace",
					fontStyle: "bold",
				},
			)
			.setScrollFactor(0)
			.setDepth(200)
			.setOrigin(0.5);

		this.oxygenText = this.add
			.text(10, 132, "O₂: 100/100", {
				fontSize: "12px",
				color: "#58a6ff",
				fontFamily: "monospace",
			})
			.setScrollFactor(0)
			.setDepth(100);

		this.ammoText = this.add
			.text(10, 148, "Ammo: --", {
				fontSize: "12px",
				color: "#ffaa00",
				fontFamily: "monospace",
			})
			.setScrollFactor(0)
			.setDepth(100);

		this.inventoryText = this.add
			.text(
				this.cameras.main.width / 2,
				this.cameras.main.height - 30,
				"Inventory: []",
				{
					fontSize: "14px",
					color: "#ffffff",
					backgroundColor: "#00000088",
					padding: { x: 10, y: 5 },
					fontFamily: "monospace",
				},
			)
			.setScrollFactor(0)
			.setDepth(100)
			.setOrigin(0.5);

		// Crosshair
		this.crosshair = this.add.graphics();
		this.crosshair.lineStyle(2, 0xff0000, 0.8);
		this.crosshair.strokeCircle(0, 0, 8);
		this.crosshair.moveTo(-12, 0);
		this.crosshair.lineTo(-4, 0);
		this.crosshair.moveTo(12, 0);
		this.crosshair.lineTo(4, 0);
		this.crosshair.moveTo(0, -12);
		this.crosshair.lineTo(0, -4);
		this.crosshair.moveTo(0, 12);
		this.crosshair.lineTo(0, 4);
		this.crosshair.strokePath();
		this.crosshair.setDepth(200);

		// Build ghost (40x40 grid)
		this.buildGhost = this.add.rectangle(0, 0, 40, 40, 0x58a6ff, 0.4);
		this.buildGhost.setStrokeStyle(2, 0x58a6ff);
		this.buildGhost.setDepth(199);
		this.buildGhost.setVisible(false);

		this.healthText = this.add
			.text(10, 64, "Health: 100/100", {
				fontSize: "16px",
				color: "#ff3333",
				fontFamily: "monospace",
				fontStyle: "bold",
			})
			.setScrollFactor(0)
			.setDepth(100);

		// --- Input processing ---
		this.input.on("pointerdown", (pointer: Phaser.Input.Pointer) => {
			if (this.isDead || this.craftPanelVisible) return;
			// Convert screen coordinates to world coordinates
			const worldPoint = this.cameras.main.getWorldPoint(pointer.x, pointer.y);

			if (this.buildMode) {
				// Click to build
				const gridX = Math.round(worldPoint.x / 40.0);
				const gridY = Math.round(worldPoint.y / 40.0);
				this.net.send({
					BuildStructure: {
						item: this.selectedStructure,
						grid_x: gridX,
						grid_y: gridY,
					},
				});
				return;
			}

			const myEntity = this.entities.get(this.myEntityId);
			if (!myEntity) return;

			// Check if mining laser is selected in hotbar
			const selectedItem = this.getSelectedHotbarItem();
			if (selectedItem === "mining_laser") {
				// Find nearest resource node as target
				let nearestId: number | null = null;
				let nearestDist = 1500.0 * 1500.0;
				for (const [id, ent] of this.entities) {
					if (id === this.myEntityId) continue;
					const distSq =
						(ent.sprite.x - myEntity.sprite.x) ** 2 +
						(ent.sprite.y - myEntity.sprite.y) ** 2;
					if (distSq < nearestDist) {
						nearestDist = distSq;
						nearestId = id;
					}
				}
				this.net.send({
					Mine: {
						target_entity_id: nearestId,
					},
				});
			} else {
				const dx = worldPoint.x - myEntity.sprite.x;
				const dy = worldPoint.y - myEntity.sprite.y;
				this.net.send({
					Fire: {
						direction: { x: dx, y: dy },
						client_latency_ms: this.net.getEstimatedRttMs(),
					},
				});
			}
		});

		// --- Network listener ---
		this.net.onMessage((msg: ServerMessage) => {
			if ("StateSnapshot" in msg) {
				this.handleSnapshot(msg.StateSnapshot);
			} else if ("WorldMapChunk" in msg) {
				this.handleWorldMapChunk(msg.WorldMapChunk);
			} else if ("ZoneChanged" in msg) {
				if ("AsteroidBase" in msg.ZoneChanged.zone) {
					this.scene.start("AsteroidBaseScene", {
						net: this.net,
						entityId: this.myEntityId,
					});
				}
			} else if ("SpawnProjectile" in msg) {
				this.projectileIds.add(msg.SpawnProjectile.entity_id);
			} else if ("HealthChanged" in msg) {
				if (msg.HealthChanged.entity_id === this.myEntityId) {
					this.currentHealth = msg.HealthChanged.health;
					this.maxHealth = msg.HealthChanged.max_health;
					this.healthText.setText(
						`Health: ${Math.max(0, Math.floor(this.currentHealth))}/${this.maxHealth}`,
					);
				}
			} else if ("OxygenChanged" in msg) {
				if (msg.OxygenChanged.entity_id === this.myEntityId) {
					this.currentOxygen = msg.OxygenChanged.current;
					this.maxOxygen = msg.OxygenChanged.max;
					const color = this.currentOxygen < 30 ? "#ff4444" : "#58a6ff";
					this.oxygenText.setText(
						`O₂: ${Math.max(0, Math.floor(this.currentOxygen))}/${this.maxOxygen}`,
					);
					this.oxygenText.setColor(color);
				}
			} else if ("PlayerDied" in msg) {
				if (msg.PlayerDied.entity_id === this.myEntityId) {
					this.isDead = true;
					this.healthText.setText(
						"DEAD - Inventory Dropped!\nPress R to Respawn",
					);
					this.healthText.setColor("#ff0000");
					this.inventoryItems = [];
					this.inventoryText.setText("Inventory: []");
					this.updateHotbarUI();
				}
				// Remove the dead entity sprite
				const deadEnt = this.entities.get(msg.PlayerDied.entity_id);
				if (deadEnt) {
					deadEnt.sprite.destroy();
					deadEnt.label.destroy();
					deadEnt.facingLine?.destroy();
					this.entities.delete(msg.PlayerDied.entity_id);
				}
			} else if ("InventoryUpdated" in msg) {
				if (msg.InventoryUpdated.entity_id === this.myEntityId) {
					try {
						this.inventoryItems = JSON.parse(msg.InventoryUpdated.items_json);
						this.inventoryText.setText(
							`Inventory: [${this.inventoryItems.join(", ")}]`,
						);
						this.updateHotbarUI();
					} catch (e) {
						console.error("Failed to parse inventory", e);
					}
				}
			} else if ("ResourceDepleted" in msg) {
				const ent = this.entities.get(msg.ResourceDepleted.entity_id);
				if (ent) {
					ent.sprite.destroy();
					ent.label.destroy();
					ent.facingLine?.destroy();
					this.entities.delete(msg.ResourceDepleted.entity_id);
				}
			} else if ("XpGained" in msg) {
				if (msg.XpGained.branch === "Extraction") {
					this.xpText.setText(
						`Extraction Lv.${msg.XpGained.new_level} (${msg.XpGained.new_total} XP)`,
					);
					this.flashText(
						this.xpFlashText,
						`+${msg.XpGained.amount} Extraction XP`,
						3000,
					);
				}
			} else if ("CraftDenied" in msg) {
				this.flashText(
					this.craftDeniedText,
					`Craft denied: ${msg.CraftDenied.reason}`,
					2000,
				);
			} else if ("TraderQuotes" in msg) {
				this.traderQuotes = msg.TraderQuotes.quotes;
			} else if ("CreditsChanged" in msg) {
				this.factionCredits = msg.CreditsChanged.balance;
				this.creditsText.setText(
					`Credits: ${this.factionCredits.toLocaleString()}`,
				);
			} else if ("TradeFailed" in msg) {
				this.flashText(
					this.tradeFailedText,
					`Trade failed: ${msg.TradeFailed.reason}`,
					2000,
				);
			} else if ("AmmoChanged" in msg) {
				if (msg.AmmoChanged.entity_id === this.myEntityId) {
					this.currentAmmo = msg.AmmoChanged.ammo;
					this.maxAmmo = msg.AmmoChanged.max_ammo;
					this.ammoText.setText(`Ammo: ${this.currentAmmo}/${this.maxAmmo}`);
					if (this.currentAmmo === 0) {
						this.ammoText.setColor("#ff4444");
					} else {
						this.ammoText.setColor("#ffaa00");
					}
				}
			} else if ("ChatMessage" in msg) {
				const channel = msg.ChatMessage.channel;
				const color = channel === "Faction" ? "#2ea043" : "#8b949e";
				const from =
					msg.ChatMessage.from_entity_id === 0
						? "SYSTEM"
						: `E${msg.ChatMessage.from_entity_id}`;
				this.addChatMessage(from, msg.ChatMessage.text, color);
			}
		});

		this.updateHotbarUI();
	}

	update(_time: number, delta: number): void {
		const me = this.entities.get(this.myEntityId);

		if (me && !this.isDead) {
			// Calculate Biome Background Color
			const pos = me.sprite;
			const dist = Math.sqrt(pos.x * pos.x + pos.y * pos.y);
			const noiseVal = this.noise2D(pos.x / 1000.0, pos.y / 1000.0);
			const perturbedDist = dist + noiseVal * 500.0;

			let targetColor = Phaser.Display.Color.HexStringToColor("#111111"); // OuterRim (Dark Gray)
			if (perturbedDist < 1500.0) {
				targetColor = Phaser.Display.Color.HexStringToColor("#330000"); // Core (Deep Red)
			} else if (perturbedDist < 3500.0) {
				targetColor = Phaser.Display.Color.HexStringToColor("#1a0a2e"); // MidZone (Purple/Blue)
			}

			// Lerp color for smooth transition
			const currentColor = Phaser.Display.Color.Interpolate.ColorWithColor(
				this.cameras.main.backgroundColor,
				targetColor,
				100,
				5,
			);
			this.cameras.main.setBackgroundColor(currentColor);
		}

		if (this.isDead) {
			// Handle respawn
			if (
				this.input.keyboard &&
				Phaser.Input.Keyboard.JustDown(
					this.input.keyboard.addKey(Phaser.Input.Keyboard.KeyCodes.R),
				)
			) {
				this.net.send({ Respawn: null });
				this.isDead = false;
				this.healthText.setColor("#ff3333");
			}
			return; // don't process movement if dead
		}

		// --- Input processing ---
		let dx = 0;
		let dy = 0;

		if (this.wasd) {
			if (this.wasd.A.isDown || this.cursors.left.isDown) dx -= 1;
			if (this.wasd.D.isDown || this.cursors.right.isDown) dx += 1;
			if (this.wasd.W.isDown || this.cursors.up.isDown) dy -= 1;
			if (this.wasd.S.isDown || this.cursors.down.isDown) dy += 1;
		}

		// Normalize diagonal movement.
		const len = Math.sqrt(dx * dx + dy * dy);
		const direction: Vec2 =
			len > 0 ? { x: dx / len, y: dy / len } : { x: 0, y: 0 };

		// Only send if direction changed.
		if (
			direction.x !== this.lastDirection.x ||
			direction.y !== this.lastDirection.y
		) {
			this.inputSeq++;
			this.net.send({
				Move: { direction, seq: this.inputSeq },
			});
			this.lastDirection = direction;
		}

		// Zone transfer on Space.
		if (this.spaceKey && Phaser.Input.Keyboard.JustDown(this.spaceKey)) {
			this.net.send({ ZoneTransfer: { target: { AsteroidBase: 1 } } });
		}

		// Crafting panel toggle on C.
		if (this.craftKey && Phaser.Input.Keyboard.JustDown(this.craftKey)) {
			this.toggleCraftPanel();
		}

		// Build mode toggle on B.
		if (this.buildKey && Phaser.Input.Keyboard.JustDown(this.buildKey)) {
			this.buildMode = !this.buildMode;
			this.buildGhost.setVisible(this.buildMode);
			this.crosshair.setVisible(!this.buildMode);
		}

		// Chat toggle on Enter.
		if (
			this.input.keyboard &&
			Phaser.Input.Keyboard.JustDown(
				this.input.keyboard.addKey(Phaser.Input.Keyboard.KeyCodes.ENTER),
			)
		) {
			this.toggleChat();
		}

		// --- Interpolation & Updates ---
		const worldPoint = this.cameras.main.getWorldPoint(
			this.input.activePointer.x,
			this.input.activePointer.y,
		);
		if (this.buildMode) {
			const gridX = Math.round(worldPoint.x / 40.0);
			const gridY = Math.round(worldPoint.y / 40.0);
			this.buildGhost.setPosition(gridX * 40, gridY * 40);
		} else {
			this.crosshair.setPosition(worldPoint.x, worldPoint.y);
		}

		for (const [id, ent] of this.entities) {
			if (id === this.myEntityId) {
				// Client-side prediction for local player
				const MAX_SPEED = 200.0;
				ent.sprite.x += direction.x * MAX_SPEED * (delta / 1000.0);
				ent.sprite.y += direction.y * MAX_SPEED * (delta / 1000.0);

				// Clamp to bounds to prevent overshooting before server corrects
				ent.sprite.x = Phaser.Math.Clamp(ent.sprite.x, -2000, 2000);
				ent.sprite.y = Phaser.Math.Clamp(ent.sprite.y, -2000, 2000);

				// Snap to server target if we are heavily desynced (e.g. wall collision)
				const distSq =
					(ent.targetX - ent.sprite.x) ** 2 + (ent.targetY - ent.sprite.y) ** 2;
				if (distSq > 150 * 150) {
					// 150 units tolerance
					ent.sprite.x = ent.targetX;
					ent.sprite.y = ent.targetY;
				}
			} else {
				// Interpolate other entities
				ent.sprite.x += (ent.targetX - ent.sprite.x) * INTERPOLATION_SPEED;
				ent.sprite.y += (ent.targetY - ent.sprite.y) * INTERPOLATION_SPEED;
			}

			ent.label.setPosition(ent.sprite.x, ent.sprite.y - 20);
			if (ent.facingLine) {
				ent.facingLine.setPosition(ent.sprite.x, ent.sprite.y);
			}
		}

		// Update my facing line
		if (!this.isDead && me?.facingLine) {
			const dxFacing = worldPoint.x - me.sprite.x;
			const dyFacing = worldPoint.y - me.sprite.y;
			const angle = Math.atan2(dyFacing, dxFacing);
			me.facingLine.setRotation(angle);
		}

		// --- HUD ---
		this.fpsText.setText(`FPS: ${Math.round(this.game.loop.actualFps)}`);
	}

	private handleWorldMapChunk(chunkMsg: {
		chunk_x: number;
		chunk_y: number;
		tiles: TileData[];
	}): void {
		const key = `${chunkMsg.chunk_x},${chunkMsg.chunk_y}`;
		if (this.chunks.has(key)) {
			// Already have this chunk — ignore duplicate.
			return;
		}

		// Clean up old chunk graphics if pool is getting large.
		if (this.chunkGraphicsPool.length > 200) {
			const old = this.chunkGraphicsPool.shift();
			if (old) old.destroy();
		}

		const graphics = this.add.graphics();
		graphics.setDepth(0);
		this.chunkGraphicsPool.push(graphics);

		const chunkWorldX = chunkMsg.chunk_x * this.chunkSize * this.tileSize;
		const chunkWorldY = chunkMsg.chunk_y * this.chunkSize * this.tileSize;

		for (let ly = 0; ly < this.chunkSize; ly++) {
			for (let lx = 0; lx < this.chunkSize; lx++) {
				const tile = chunkMsg.tiles[ly * this.chunkSize + lx];
				if (!tile || tile.terrain === "Empty") continue;

				const tx = chunkWorldX + lx * this.tileSize;
				const ty = chunkWorldY + ly * this.tileSize;

				let color = 0x0d1117;
				switch (tile.terrain) {
					case "Rock":
						color = 0x3d444d;
						break;
					case "DeepRavine":
						color = 0x000000;
						break;
					case "AsteroidField":
						color = 0x21262d;
						break;
					case "AutomataSpire":
						color = 0x7d1a1a;
						break;
				}

				// Height-based brightness adjustment
				const brightness = 0.7 + (tile.height / 255) * 0.3;
				const r = ((color >> 16) & 0xff) * brightness;
				const g = ((color >> 8) & 0xff) * brightness;
				const b = (color & 0xff) * brightness;
				const adjustedColor = Phaser.Display.Color.GetColor(r, g, b);

				graphics.fillStyle(adjustedColor, 1.0);
				graphics.fillRect(tx, ty, this.tileSize, this.tileSize);

				// Enclosed ceiling indicator — subtle border
				if (tile.ceiling === "Enclosed") {
					graphics.lineStyle(1, 0x58a6ff, 0.3);
					graphics.strokeRect(tx, ty, this.tileSize, this.tileSize);
				}
			}
		}

		this.chunks.set(key, { tiles: chunkMsg.tiles, graphics: [graphics] });
	}

	private handleSnapshot(snapshot: {
		tick: number;
		last_processed_seq: number;
		entities: EntitySnapshot[];
	}): void {
		this.tickText.setText(`Tick: ${snapshot.tick}`);

		const seenIds = new Set<number>();

		for (const entity of snapshot.entities) {
			seenIds.add(entity.id);

			const existing = this.entities.get(entity.id);
			if (existing) {
				existing.targetX = entity.position.x;
				existing.targetY = entity.position.y;
			} else {
				this.spawnEntity(entity);
			}
		}

		// Remove entities that are no longer in the snapshot.
		for (const [id, ent] of this.entities) {
			if (!seenIds.has(id)) {
				ent.sprite.destroy();
				ent.label.destroy();
				ent.facingLine?.destroy();
				this.entities.delete(id);
			}
		}

		// Camera follow.
		const me = this.entities.get(this.myEntityId);
		if (me) {
			this.cameras.main.centerOn(me.sprite.x, me.sprite.y);
			this.minimap.centerOn(me.sprite.x, me.sprite.y);
		}
	}

	private spawnEntity(entity: EntitySnapshot): void {
		const isMe = entity.id === this.myEntityId;
		const type = entity.entity_type;

		let color = 0xffffff;
		let size = 14;
		let labelText = "Entity";
		let labelColor = "#8b949e";

		if (type === "Player") {
			color = isMe ? 0x58a6ff : 0x7c3aed;
			labelText = isMe ? "YOU" : `Player ${entity.id}`;
			labelColor = isMe ? "#58a6ff" : "#7c3aed";
		} else if (type === "Hostile") {
			color = 0xff4444;
			labelText = "Automata";
			labelColor = "#ff4444";
		} else if (type === "Trader") {
			color = 0x2ea043;
			labelText = "Trader";
			labelColor = "#2ea043";
		} else if (type === "ResourceNode") {
			color = 0x8b949e;
			labelText = "Ore";
			labelColor = "#8b949e";
		} else if (type === "Item") {
			color = 0xd2a8ff;
			size = 6;
			labelText = "";
		} else if (type === "Unknown") {
			// Used for base structures for now
			color = 0x888888;
			size = 20; // 40x40 tile
			labelText = "";
		} else if (type === "Projectile") {
			color = 0xffa500;
			size = 5;
			labelText = "";
		}

		let sprite: Phaser.GameObjects.Shape;
		if (type === "Unknown") {
			// Base tile rectangle
			sprite = this.add.rectangle(
				entity.position.x,
				entity.position.y,
				40,
				40,
				color,
				1.0,
			);
			sprite.setStrokeStyle(1, 0x000000);
		} else {
			sprite = this.add.circle(
				entity.position.x,
				entity.position.y,
				size,
				color,
			);
		}
		sprite.setDepth(10);

		// If it's a trader, make it interactive to open trade UI
		if (type === "Trader") {
			sprite.setInteractive({ useHandCursor: true });
			sprite.on("pointerdown", () => {
				this.openTradeUI(entity.id);
			});
		}

		const label = this.add
			.text(entity.position.x, entity.position.y - size - 10, labelText, {
				fontSize: "10px",
				color: labelColor,
				fontFamily: "monospace",
			})
			.setOrigin(0.5)
			.setDepth(10);

		let facingLine: Phaser.GameObjects.Graphics | undefined;
		if (isMe) {
			facingLine = this.add.graphics();
			facingLine.lineStyle(2, 0xffffff, 0.8);
			facingLine.moveTo(0, 0);
			facingLine.lineTo(20, 0);
			facingLine.strokePath();
			facingLine.setDepth(11);
		}

		this.entities.set(entity.id, {
			sprite,
			facingLine,
			label,
			targetX: entity.position.x,
			targetY: entity.position.y,
		});
	}

	private toggleChat(): void {
		if (this.chatVisible) {
			// Send message if there's text.
			if (this.chatInput.trim().length > 0) {
				this.net.send({
					Chat: { channel: "Global", text: this.chatInput.trim() },
				});
			}
			this.chatInput = "";
			this.chatVisible = false;
			for (const el of this.chatElements) {
				el.destroy();
			}
			this.chatElements = [];
		} else {
			this.chatVisible = true;
			this.renderChatUI();
		}
	}

	private addChatMessage(from: string, text: string, color: string): void {
		this.chatMessages.push({ from, text, color });
		if (this.chatMessages.length > 50) {
			this.chatMessages.shift();
		}
		if (this.chatVisible) {
			this.renderChatUI();
		}
	}

	private renderChatUI(): void {
		for (const el of this.chatElements) {
			el.destroy();
		}
		this.chatElements = [];

		const cx = 20;
		const cy = this.cameras.main.height - 200;
		const panelW = 400;
		const panelH = 160;

		const bg = this.add.rectangle(
			cx + panelW / 2,
			cy + panelH / 2,
			panelW,
			panelH,
			0x000000,
			0.85,
		);
		bg.setStrokeStyle(1, 0x333333);
		bg.setScrollFactor(0);
		bg.setDepth(200);
		this.chatElements.push(bg);

		let yOff = cy + panelH - 10;
		const recent = this.chatMessages.slice(-6);
		for (let i = recent.length - 1; i >= 0; i--) {
			const msg = recent[i];
			const line = this.add
				.text(cx + 10, yOff, `[${msg.from}] ${msg.text}`, {
					fontSize: "11px",
					color: msg.color,
					fontFamily: "monospace",
					wordWrap: { width: panelW - 20 },
				})
				.setScrollFactor(0)
				.setDepth(201);
			this.chatElements.push(line);
			yOff -= 18;
		}

		const inputLine = this.add
			.text(cx + 10, cy + panelH + 5, `> ${this.chatInput}_`, {
				fontSize: "12px",
				color: "#ffffff",
				fontFamily: "monospace",
			})
			.setScrollFactor(0)
			.setDepth(201);
		this.chatElements.push(inputLine);
	}

	private tradeUIElements: Phaser.GameObjects.GameObject[] = [];

	private closeTradeUI(): void {
		for (const el of this.tradeUIElements) {
			el.destroy();
		}
		this.tradeUIElements = [];
	}

	private openTradeUI(traderId: number): void {
		if (this.isDead) return;
		this.closeTradeUI();

		// Request fresh quotes when opening trade UI.
		this.net.send("RequestTraderQuotes");

		const cx = this.cameras.main.width / 2;
		const cy = this.cameras.main.height / 2;
		const panelW = 360;
		const panelH = 420;

		const uiBg = this.add.rectangle(cx, cy, panelW, panelH, 0x000000, 0.92);
		uiBg.setStrokeStyle(2, 0x2ea043);
		uiBg.setScrollFactor(0);
		uiBg.setDepth(300);
		this.tradeUIElements.push(uiBg);

		const title = this.add
			.text(cx, cy - panelH / 2 + 20, `TRADER E${traderId}`, {
				fontSize: "16px",
				color: "#2ea043",
				fontFamily: "monospace",
				fontStyle: "bold",
			})
			.setOrigin(0.5)
			.setScrollFactor(0)
			.setDepth(301);
		this.tradeUIElements.push(title);

		const creditsLabel = this.add
			.text(
				cx,
				cy - panelH / 2 + 42,
				`Faction: ${this.factionCredits.toLocaleString()} CR`,
				{
					fontSize: "11px",
					color: "#f0a000",
					fontFamily: "monospace",
				},
			)
			.setOrigin(0.5)
			.setScrollFactor(0)
			.setDepth(301);
		this.tradeUIElements.push(creditsLabel);

		// Filter quotes for this trader.
		const quotes = this.traderQuotes.filter(
			(q) => q.trader_entity_id === traderId,
		);

		let yOff = cy - panelH / 2 + 70;
		for (const q of quotes) {
			const itemLabel = this.add
				.text(cx - panelW / 2 + 20, yOff, q.item.toUpperCase(), {
					fontSize: "12px",
					color: "#ffffff",
					fontFamily: "monospace",
				})
				.setScrollFactor(0)
				.setDepth(301);
			this.tradeUIElements.push(itemLabel);

			const buyLabel = this.add
				.text(cx - 30, yOff, `BUY ${Math.floor(q.sell_price)}`, {
					fontSize: "11px",
					color: "#58a6ff",
					fontFamily: "monospace",
					backgroundColor: "#1a2332",
					padding: { x: 6, y: 2 },
				})
				.setOrigin(0.5)
				.setScrollFactor(0)
				.setDepth(301)
				.setInteractive({ useHandCursor: true });
			this.tradeUIElements.push(buyLabel);

			buyLabel.on("pointerover", () => buyLabel.setColor("#ffffff"));
			buyLabel.on("pointerout", () => buyLabel.setColor("#58a6ff"));
			buyLabel.on("pointerdown", () => {
				this.net.send({
					BuyItem: { item: q.item, quantity: 1 },
				});
				buyLabel.setColor("#2ea043");
				this.time.delayedCall(200, () => buyLabel.setColor("#58a6ff"));
			});

			const sellLabel = this.add
				.text(cx + 70, yOff, `SELL ${Math.floor(q.buy_price)}`, {
					fontSize: "11px",
					color: "#2ea043",
					fontFamily: "monospace",
					backgroundColor: "#1a2332",
					padding: { x: 6, y: 2 },
				})
				.setOrigin(0.5)
				.setScrollFactor(0)
				.setDepth(301)
				.setInteractive({ useHandCursor: true });
			this.tradeUIElements.push(sellLabel);

			sellLabel.on("pointerover", () => sellLabel.setColor("#ffffff"));
			sellLabel.on("pointerout", () => sellLabel.setColor("#2ea043"));
			sellLabel.on("pointerdown", () => {
				this.net.send({
					SellItem: { item: q.item, quantity: 1 },
				});
				sellLabel.setColor("#58a6ff");
				this.time.delayedCall(200, () => sellLabel.setColor("#2ea043"));
			});

			yOff += 28;
		}

		const closeBtn = this.add
			.text(cx, cy + panelH / 2 - 25, "[ CLOSE ]", {
				fontSize: "13px",
				color: "#ff4444",
				fontFamily: "monospace",
			})
			.setOrigin(0.5)
			.setScrollFactor(0)
			.setDepth(301)
			.setInteractive({ useHandCursor: true });
		this.tradeUIElements.push(closeBtn);

		closeBtn.on("pointerdown", () => this.closeTradeUI());
	}

	private getSelectedHotbarItem(): string | null {
		if (this.hotbarSlot === 0) return null;
		return this.inventoryItems[this.hotbarSlot - 1] ?? null;
	}

	private updateHotbarUI(): void {
		// Clear old hotbar elements
		for (const el of this.hotbarElements) {
			el.destroy();
		}
		this.hotbarElements = [];

		const slotSize = 40;
		const spacing = 4;
		const totalWidth = 9 * (slotSize + spacing);
		const startX = (this.cameras.main.width - totalWidth) / 2 + slotSize / 2;
		const y = this.cameras.main.height - 70;

		for (let i = 0; i <= 9; i++) {
			const x = startX + i * (slotSize + spacing);
			const isSelected = this.hotbarSlot === i;
			const bg = this.add.rectangle(x, y, slotSize, slotSize, 0x000000, 0.85);
			bg.setStrokeStyle(isSelected ? 3 : 1, isSelected ? 0x58a6ff : 0x333333);
			bg.setScrollFactor(0);
			bg.setDepth(100);
			this.hotbarElements.push(bg);

			let labelText = i === 0 ? "🔫" : `${i}`;
			if (i > 0 && i <= this.inventoryItems.length) {
				const item = this.inventoryItems[i - 1];
				// Show first 2 chars of item name as abbreviation
				labelText = item.slice(0, 2).toUpperCase();
			}

			const label = this.add
				.text(x, y, labelText, {
					fontSize: "11px",
					color: isSelected ? "#58a6ff" : "#888888",
					fontFamily: "monospace",
				})
				.setOrigin(0.5)
				.setScrollFactor(0)
				.setDepth(101);
			this.hotbarElements.push(label);
		}
	}

	private flashText(
		textObj: Phaser.GameObjects.Text,
		message: string,
		durationMs: number,
	): void {
		textObj.setText(message);
		textObj.setAlpha(1);
		this.tweens.killTweensOf(textObj);
		this.tweens.add({
			targets: textObj,
			alpha: 0,
			duration: durationMs,
			ease: "Power2",
		});
	}

	private toggleCraftPanel(): void {
		if (this.craftPanelVisible) {
			// Close the panel
			for (const el of this.craftPanelElements) {
				el.destroy();
			}
			this.craftPanelElements = [];
			this.craftPanelVisible = false;
			return;
		}

		this.craftPanelVisible = true;
		const cx = this.cameras.main.width - 180;
		const cy = 180;
		const panelW = 320;
		const panelH = 400;

		const bg = this.add.rectangle(
			cx,
			cy + panelH / 2 - 10,
			panelW,
			panelH,
			0x000000,
			0.92,
		);
		bg.setStrokeStyle(2, 0x58a6ff);
		bg.setScrollFactor(0);
		bg.setDepth(300);
		this.craftPanelElements.push(bg);

		const title = this.add
			.text(cx, cy - 10, "⚙ FABRICATOR", {
				fontSize: "16px",
				color: "#58a6ff",
				fontFamily: "monospace",
				fontStyle: "bold",
			})
			.setOrigin(0.5)
			.setScrollFactor(0)
			.setDepth(301);
		this.craftPanelElements.push(title);

		// Recipe definitions (must match server-side RECIPES)
		const recipes = [
			{ name: "iron_plate", inputs: "2× iron", output: "iron_plate" },
			{ name: "copper_wire", inputs: "2× copper", output: "copper_wire" },
			{
				name: "circuit",
				inputs: "copper_wire + iron_plate",
				output: "circuit",
			},
			{ name: "glass", inputs: "2× silica", output: "glass" },
			{ name: "medkit", inputs: "biomass + glass", output: "medkit" },
			{ name: "bio_fuel", inputs: "2× biomass", output: "bio_fuel" },
			{
				name: "titanium_plate",
				inputs: "2× titanium",
				output: "titanium_plate",
			},
			{
				name: "battery",
				inputs: "titanium_plate + circuit",
				output: "battery",
			},
			{
				name: "shield_module",
				inputs: "titanium_plate + battery + circuit",
				output: "shield_module",
			},
			{ name: "wall_segment", inputs: "2× iron_plate", output: "wall_segment" },
			{ name: "door", inputs: "iron_plate + circuit", output: "door" },
			{
				name: "generator",
				inputs: "battery + titanium_plate + circuit",
				output: "generator",
			},
			{
				name: "workbench",
				inputs: "2× iron_plate + circuit",
				output: "workbench",
			},
		];

		let yOff = cy + 20;
		for (const r of recipes) {
			const btn = this.add
				.text(cx, yOff, `▸ ${r.output}`, {
					fontSize: "13px",
					color: "#8b949e",
					fontFamily: "monospace",
				})
				.setOrigin(0.5)
				.setScrollFactor(0)
				.setDepth(301)
				.setInteractive({ useHandCursor: true });

			const detail = this.add
				.text(cx, yOff + 14, `  ${r.inputs}`, {
					fontSize: "10px",
					color: "#555555",
					fontFamily: "monospace",
				})
				.setOrigin(0.5)
				.setScrollFactor(0)
				.setDepth(301);

			btn.on("pointerover", () => btn.setColor("#58a6ff"));
			btn.on("pointerout", () => btn.setColor("#8b949e"));
			btn.on("pointerdown", () => {
				this.net.send({ Craft: { recipe: r.name } });
				btn.setColor("#2ea043");
				this.time.delayedCall(300, () => btn.setColor("#8b949e"));
			});

			this.craftPanelElements.push(btn, detail);
			yOff += 34;
		}

		// Close button
		const closeBtn = this.add
			.text(cx, yOff + 10, "[ CLOSE ]", {
				fontSize: "13px",
				color: "#ff4444",
				fontFamily: "monospace",
			})
			.setOrigin(0.5)
			.setScrollFactor(0)
			.setDepth(301)
			.setInteractive({ useHandCursor: true });
		closeBtn.on("pointerdown", () => this.toggleCraftPanel());
		this.craftPanelElements.push(closeBtn);
	}
}
