/**
 * AsteroidBaseScene — A faction's private asteroid base.
 *
 * Renders entities from server snapshots including base structures.
 */

import * as Phaser from "phaser";
import { resolveSprite } from "../config/SpriteRegistry";
import type { NetworkClient } from "../net/NetworkClient";
import type { EntitySnapshot, ServerMessage } from "../types/protocol";
import { gameUi } from "../ui/GameUiManager";

interface BaseData {
	net: NetworkClient;
	entityId: number;
}

interface RenderedEntity {
	sprite: Phaser.GameObjects.Sprite;
	label: Phaser.GameObjects.Text;
	targetX: number;
	targetY: number;
}

export class AsteroidBaseScene extends Phaser.Scene {
	private net!: NetworkClient;
	private myEntityId!: number;
	private spaceKey!: Phaser.Input.Keyboard.Key;
	private entities: Map<number, RenderedEntity> = new Map();

	constructor() {
		super({ key: "AsteroidBaseScene" });
	}

	init(data: BaseData): void {
		this.net = data.net;
		this.myEntityId = data.entityId;
	}

	create(): void {
		gameUi.showGameplay();
		gameUi.updateSceneContext("ASTEROID BASE", "[SPACE] Return to Overworld");
		this.cameras.main.setBackgroundColor("#1a0a2e");

		// --- Input ---
		if (this.input.keyboard) {
			this.spaceKey = this.input.keyboard.addKey(
				Phaser.Input.Keyboard.KeyCodes.SPACE,
			);
		}

		// Network listener.
		this.net.onMessage((msg: ServerMessage) => {
			if (!this.scene.isActive(this.scene.key)) return;
			if ("StateSnapshot" in msg) {
				this.handleSnapshot(msg.StateSnapshot.entities);
			} else if ("ZoneChanged" in msg) {
				if ("Overworld" in msg.ZoneChanged.zone) {
					this.scene.start("OverworldScene", {
						net: this.net,
						entityId: this.myEntityId,
					});
				}
			} else if ("OxygenChanged" in msg) {
				if (msg.OxygenChanged.entity_id === this.myEntityId) {
					gameUi.updateVitals({
						health: "--",
						oxygen: `${Math.max(0, Math.floor(msg.OxygenChanged.current))}/${msg.OxygenChanged.max}`,
						ammo: "--",
						credits: "--",
						fps: `${Math.round(this.game.loop.actualFps)}`,
						tick: "--",
					});
				}
			} else if ("PowerState" in msg) {
				gameUi.updatePowerState(msg.PowerState.networks);
			}
		});
		this.net.send("RequestPowerState");
	}

	update(): void {
		if (this.spaceKey && Phaser.Input.Keyboard.JustDown(this.spaceKey)) {
			this.net.send({
				ZoneTransfer: { target: { Overworld: null } },
			});
		}

		// Interpolate entities toward their server targets.
		for (const ent of this.entities.values()) {
			ent.sprite.x += (ent.targetX - ent.sprite.x) * 0.2;
			ent.sprite.y += (ent.targetY - ent.sprite.y) * 0.2;
			ent.label.setPosition(ent.sprite.x, ent.sprite.y - 20);
		}
	}

	private handleSnapshot(entities: EntitySnapshot[]): void {
		const seenIds = new Set<number>();

		for (const entity of entities) {
			seenIds.add(entity.id);
			const existing = this.entities.get(entity.id);
			if (existing) {
				existing.targetX = entity.position.x;
				existing.targetY = entity.position.y;
			} else {
				this.spawnEntity(entity);
			}
		}

		// Remove entities no longer in snapshot.
		for (const [id, ent] of this.entities) {
			if (!seenIds.has(id)) {
				ent.sprite.destroy();
				ent.label.destroy();
				this.entities.delete(id);
			}
		}

		// Update power status based on nearby generators.
		let structures = 0;
		for (const entity of entities) {
			if (entity.entity_type === "Unknown") {
				// Base structure - could be generator or consumer.
				// We can't tell from the snapshot alone, so count all structures.
				structures += 1;
			}
		}
		gameUi.updateBaseStatus(structures);
	}

	private spawnEntity(entity: EntitySnapshot): void {
		const isMe = entity.id === this.myEntityId;
		const type = entity.entity_type;

		const spriteDef = resolveSprite(type);
		let sprite: Phaser.GameObjects.Sprite;

		if (type === "Unknown") {
			// Structures use the structure texture
			sprite = this.add.sprite(
				entity.position.x,
				entity.position.y,
				spriteDef.texture,
			);
			sprite.setScale(spriteDef.scale ?? 1.0);
		} else {
			// Standard entity sprite
			sprite = this.add.sprite(
				entity.position.x,
				entity.position.y,
				spriteDef.texture,
			);
			sprite.setScale(spriteDef.scale ?? 1.0);
		}

		// Apply tint for differentiation
		if (type === "Player" && isMe) {
			sprite.setTint(0x58a6ff); // Blue for self
		} else if (type === "Player") {
			sprite.setTint(0x7c3aed); // Purple for others
		}

		sprite.setDepth(10);

		const label = this.add
			.text(
				entity.position.x,
				entity.position.y - 30,
				isMe ? "YOU" : `P${entity.id}`,
				{
					fontSize: "10px",
					color: isMe ? "#58a6ff" : "#aaaaaa",
					fontFamily: "monospace",
				},
			)
			.setOrigin(0.5)
			.setDepth(11);

		this.entities.set(entity.id, {
			sprite,
			label,
			targetX: entity.position.x,
			targetY: entity.position.y,
		});
	}
}
