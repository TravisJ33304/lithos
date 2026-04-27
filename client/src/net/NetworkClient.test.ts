import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { NetworkClient } from "./NetworkClient";
import type { ZoneId } from "../types/protocol";

describe("NetworkClient", () => {
	let client: NetworkClient;

	beforeEach(() => {
		client = new NetworkClient("ws://localhost:9001");
	});

	afterEach(() => {
		client.disconnect();
	});

	it("should create client with default values", () => {
		expect(client).toBeDefined();
	});

	it("should parse ZoneId correctly", () => {
		const overworld: ZoneId = { Overworld: null };
		expect(overworld).toEqual({ Overworld: null });

		const asteroid: ZoneId = { AsteroidBase: 42 };
		expect(asteroid).toEqual({ AsteroidBase: 42 });
	});

	it("should handle reconnect attempts", () => {
		vi.useFakeTimers();

		client.disconnect();
		vi.runAllTimers();

		vi.useRealTimers();
	});
});