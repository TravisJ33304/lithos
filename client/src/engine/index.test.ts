import { describe, it, expect } from "vitest";
import * as engine from "./index";

describe("GameEngine", () => {
	it("should export engine module", () => {
		expect(engine).toBeDefined();
	});

	it("should export IRenderer type", () => {
		type Check = engine.IRenderer;
		expect(true).toBe(true);
	});

	it("should export IInputManager type", () => {
		type Check = engine.IInputManager;
		expect(true).toBe(true);
	});
});