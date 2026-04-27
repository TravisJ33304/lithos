/**
 * OverworldScene — The main shared game world.
 *
 * Handles player movement, entity rendering, interpolation, and zone transitions.
 */

import * as Phaser from "phaser";
import { createNoise2D } from "simplex-noise";
import type { NetworkClient } from "../net/NetworkClient";
import type { EntitySnapshot, ServerMessage, Vec2 } from "../types/protocol";

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
		}

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
			if (myEntity) {
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
			} else if ("PlayerDied" in msg) {
				if (msg.PlayerDied.entity_id === this.myEntityId) {
					this.isDead = true;
					this.healthText.setText("DEAD - Press R to Respawn");
					this.healthText.setColor("#ff0000");
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
					} catch (e) {
						console.error("Failed to parse inventory", e);
					}
				}
			}
		});
	}

	update(_time: number, _delta: number): void {
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
				this.net.send("Respawn");
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

		for (const [_id, ent] of this.entities) {
			ent.sprite.x += (ent.targetX - ent.sprite.x) * INTERPOLATION_SPEED;
			ent.sprite.y += (ent.targetY - ent.sprite.y) * INTERPOLATION_SPEED;
			ent.label.setPosition(ent.sprite.x, ent.sprite.y - 20);
			if (ent.facingLine) {
				ent.facingLine.setPosition(ent.sprite.x, ent.sprite.y);
			}
		}

		// Update my facing line
		if (!this.isDead) {
			const me = this.entities.get(this.myEntityId);
			if (me?.facingLine) {
				const dx = worldPoint.x - me.sprite.x;
				const dy = worldPoint.y - me.sprite.y;
				const angle = Math.atan2(dy, dx);
				me.facingLine.setRotation(angle);
			}
		}

		// --- HUD ---
		this.fpsText.setText(`FPS: ${Math.round(this.game.loop.actualFps)}`);
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

	private openTradeUI(traderId: number): void {
		// MVP: Simple alert / text overlay to simulate a Trade Dialog
		if (this.isDead) return;

		const uiBg = this.add.rectangle(
			this.cameras.main.width / 2,
			this.cameras.main.height / 2,
			300,
			200,
			0x000000,
			0.9,
		);
		uiBg.setStrokeStyle(2, 0x2ea043);
		uiBg.setScrollFactor(0);
		uiBg.setDepth(300);

		const title = this.add
			.text(
				this.cameras.main.width / 2,
				this.cameras.main.height / 2 - 80,
				`Scrapper Colony Trader E${traderId}`,
				{
					fontSize: "16px",
					color: "#2ea043",
					fontFamily: "monospace",
				},
			)
			.setOrigin(0.5)
			.setScrollFactor(0)
			.setDepth(301);

		const text = this.add
			.text(
				this.cameras.main.width / 2,
				this.cameras.main.height / 2 - 20,
				"Trade functionality\ncoming soon...",
				{
					fontSize: "14px",
					color: "#ffffff",
					fontFamily: "monospace",
					align: "center",
				},
			)
			.setOrigin(0.5)
			.setScrollFactor(0)
			.setDepth(301);

		const closeBtn = this.add
			.text(
				this.cameras.main.width / 2,
				this.cameras.main.height / 2 + 50,
				"[ CLOSE ]",
				{
					fontSize: "14px",
					color: "#ff4444",
					fontFamily: "monospace",
				},
			)
			.setOrigin(0.5)
			.setScrollFactor(0)
			.setDepth(301)
			.setInteractive({ useHandCursor: true });

		closeBtn.on("pointerdown", () => {
			uiBg.destroy();
			title.destroy();
			text.destroy();
			closeBtn.destroy();
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
			{ name: "circuit", inputs: "iron + iron_plate", output: "circuit" },
			{ name: "medkit", inputs: "scrap + circuit", output: "medkit" },
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
				inputs: "titan_plate + battery + circuit",
				output: "shield_module",
			},
			{ name: "wall_segment", inputs: "2× iron_plate", output: "wall_segment" },
			{ name: "door", inputs: "iron_plate + circuit", output: "door" },
			{
				name: "generator",
				inputs: "battery + titan_plate + circuit",
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
