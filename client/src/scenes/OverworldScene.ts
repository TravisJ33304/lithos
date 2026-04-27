/**
 * OverworldScene — The main shared game world.
 *
 * Handles player movement, entity rendering, interpolation, and zone transitions.
 */

import * as Phaser from "phaser";
import type { NetworkClient } from "../net/NetworkClient";
import type { EntitySnapshot, ServerMessage, Vec2 } from "../types/protocol";

/** Data passed from BootScene. */
interface OverworldData {
	net: NetworkClient;
	playerId: string;
	entityId: number;
	zone: { Overworld: null } | { AsteroidBase: number };
}

/** Runtime state for a rendered entity. */
interface RenderedEntity {
	sprite: Phaser.GameObjects.Arc;
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

	constructor() {
		super({ key: "OverworldScene" });
	}

	init(data: OverworldData): void {
		this.net = data.net;
		this.myEntityId = data.entityId;
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
			if (this.isDead) return;
			// Convert screen coordinates to world coordinates
			const worldPoint = this.cameras.main.getWorldPoint(pointer.x, pointer.y);
			const myEntity = this.entities.get(this.myEntityId);
			if (myEntity) {
				const dx = worldPoint.x - myEntity.sprite.x;
				const dy = worldPoint.y - myEntity.sprite.y;
				this.net.send({
					Fire: {
						direction: { x: dx, y: dy },
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
						`Health: ${Math.max(0, Math.floor(this.currentHealth))}/${this.maxHealth}`
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
					this.entities.delete(msg.PlayerDied.entity_id);
				}
			}
		});
	}

	update(_time: number, _delta: number): void {
		if (this.isDead) {
			// Handle respawn
			if (this.input.keyboard && Phaser.Input.Keyboard.JustDown(this.input.keyboard.addKey(Phaser.Input.Keyboard.KeyCodes.R))) {
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

		// --- Interpolation ---
		for (const [_id, ent] of this.entities) {
			ent.sprite.x += (ent.targetX - ent.sprite.x) * INTERPOLATION_SPEED;
			ent.sprite.y += (ent.targetY - ent.sprite.y) * INTERPOLATION_SPEED;
			ent.label.setPosition(ent.sprite.x, ent.sprite.y - 20);
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
				this.entities.delete(id);
			}
		}

		// Camera follow.
		const me = this.entities.get(this.myEntityId);
		if (me) {
			this.cameras.main.centerOn(me.sprite.x, me.sprite.y);
		}
	}

	private spawnEntity(entity: EntitySnapshot): void {
		const isMe = entity.id === this.myEntityId;
		const isProjectile = this.projectileIds.has(entity.id);

		let radius = 12;
		let color = 0x8b949e;
		if (isMe) {
			radius = 14;
			color = 0x58a6ff;
		} else if (isProjectile) {
			radius = 5;
			color = 0xffa500; // Orange
		}

		const sprite = this.add.circle(
			entity.position.x,
			entity.position.y,
			radius,
			color,
		);
		sprite.setDepth(10);

		const label = this.add
			.text(
				entity.position.x,
				entity.position.y - 20,
				isProjectile ? "" : isMe ? "YOU" : `E${entity.id}`,
				{
					fontSize: "10px",
					color: isMe ? "#58a6ff" : "#8b949e",
					fontFamily: "monospace",
				},
			)
			.setOrigin(0.5)
			.setDepth(10);

		this.entities.set(entity.id, {
			sprite,
			label,
			targetX: entity.position.x,
			targetY: entity.position.y,
		});
	}
}
