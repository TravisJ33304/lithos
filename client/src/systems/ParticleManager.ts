// client/src/systems/ParticleManager.ts
import * as Phaser from "phaser";

export class ParticleManager {
	private scene: Phaser.Scene;

	constructor(scene: Phaser.Scene) {
		this.scene = scene;
	}

	createExplosion(x: number, y: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_explosion", {
			speed: { min: 50, max: 150 },
			scale: { start: 0.8, end: 0 },
			alpha: { start: 1, end: 0 },
			lifespan: 600,
			quantity: 8,
			blendMode: Phaser.BlendModes.ADD,
		});
		emitter.explode();
	}

	createMuzzleFlash(x: number, y: number, angle: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_muzzle_flash", {
			speed: 80,
			scale: { start: 0.6, end: 0 },
			lifespan: 150,
			quantity: 3,
			angle: { min: angle - 15, max: angle + 15 },
			blendMode: Phaser.BlendModes.ADD,
		});
		emitter.explode();
	}

	createHitSpark(x: number, y: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_hit_spark", {
			speed: { min: 30, max: 80 },
			scale: { start: 0.5, end: 0 },
			lifespan: 200,
			quantity: 5,
			blendMode: Phaser.BlendModes.ADD,
		});
		emitter.explode();
	}

	createMiningSparks(x: number, y: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_mining_spark", {
			speed: { min: 20, max: 60 },
			scale: { start: 0.8, end: 0 },
			lifespan: 300,
			quantity: 4,
			frequency: 100,
			blendMode: Phaser.BlendModes.ADD,
		});
		// Auto-stop after a short burst
		this.scene.time.delayedCall(400, () => emitter.stop());
	}

	createDeathPoof(x: number, y: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_smoke_puff", {
			speed: { min: 10, max: 40 },
			scale: { start: 0.7, end: 0 },
			alpha: { start: 0.8, end: 0 },
			lifespan: 800,
			quantity: 6,
		});
		emitter.explode();
	}

	createFireDot(
		x: number,
		y: number,
	): Phaser.GameObjects.Particles.ParticleEmitter {
		const emitter = this.scene.add.particles(x, y, "fx_fire_dot", {
			speed: { min: 5, max: 15 },
			scale: { start: 0.5, end: 0 },
			lifespan: 500,
			frequency: 100,
			quantity: 1,
			blendMode: Phaser.BlendModes.ADD,
		});
		return emitter;
	}

	createWarpEffect(x: number, y: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_warp_ring", {
			speed: 0,
			scale: { start: 0.2, end: 2.0 },
			alpha: { start: 0.8, end: 0 },
			lifespan: 1000,
			quantity: 1,
			blendMode: Phaser.BlendModes.ADD,
		});
		emitter.explode();
	}
}
