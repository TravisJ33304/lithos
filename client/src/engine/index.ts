/**
 * Engine Abstraction Layer
 *
 * Provides a thin interface over the rendering engine (currently Phaser 3).
 * Game logic should use these abstractions rather than calling Phaser APIs
 * directly, making it possible to swap rendering backends in the future.
 */

export interface IRenderer {
	/** Width of the game viewport in pixels. */
	readonly width: number;
	/** Height of the game viewport in pixels. */
	readonly height: number;
}

export interface IInputManager {
	/** Returns true if the given key is currently held down. */
	isKeyDown(key: string): boolean;
	/** Returns the current mouse/pointer position in world coordinates. */
	getPointerPosition(): { x: number; y: number };
}

// Concrete Phaser-backed implementations will be added as the engine layer
// is built out during Phase 1.
