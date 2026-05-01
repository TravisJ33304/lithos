/**
 * Lithos — Game Client Entry Point
 *
 * Initializes Phaser 3 and boots the game.
 */

import * as Phaser from "phaser";
import { AsteroidBaseScene } from "./scenes/AsteroidBaseScene";
import { BootScene } from "./scenes/BootScene";
import { LoginScene } from "./scenes/LoginScene";
import { OverworldScene } from "./scenes/OverworldScene";

declare global {
	interface Window {
		__PHASER_GAME__?: Phaser.Game;
	}
}

const config: Phaser.Types.Core.GameConfig = {
	type: Phaser.AUTO,
	width: 1280,
	height: 720,
	parent: "game-container",
	backgroundColor: "#0a0a1a",
	dom: {
		createContainer: true,
	},
	physics: {
		default: "arcade",
		arcade: {
			gravity: { x: 0, y: 0 },
			debug: false,
		},
	},
	scene: [BootScene, LoginScene, OverworldScene, AsteroidBaseScene],
	scale: {
		mode: Phaser.Scale.FIT,
		autoCenter: Phaser.Scale.CENTER_BOTH,
	},
	pixelArt: false,
	roundPixels: false,
};

const game = new Phaser.Game(config);
window.__PHASER_GAME__ = game;
