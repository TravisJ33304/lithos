import { describe, expect, it } from "vitest";
import {
	decodeServerMessage,
	normalizeServerMessage,
} from "./decodeServerMessage";

function hexToBytes(hex: string): Uint8Array {
	const pairs = hex.match(/../g);
	if (!pairs) throw new Error("invalid hex");
	return Uint8Array.from(pairs.map((b) => parseInt(b, 16)));
}

describe("decodeServerMessage (compact Rust wire)", () => {
	it("decodes Pong fixture", () => {
		const msg = decodeServerMessage(hexToBytes("81a4506f6e67920102"));
		expect(msg).toEqual({
			Pong: { client_timestamp: 1, server_timestamp: 2 },
		});
	});

	it("decodes JoinAck fixture", () => {
		const msg = decodeServerMessage(
			hexToBytes(
				"81a74a6f696e41636b94c410000102030405060708090a0b0c0d0e0f01a94f766572776f726c642a",
			),
		);
		expect(msg).toEqual({
			JoinAck: {
				player_id: "00010203-0405-0607-0809-0a0b0c0d0e0f",
				entity_id: 1,
				zone: { Overworld: null },
				world_seed: 42,
			},
		});
	});

	it("decodes StateSnapshot fixture", () => {
		const msg = decodeServerMessage(
			hexToBytes(
				"81ad5374617465536e617073686f7493030491950592ca3f800000ca4000000092ca00000000ca00000000a94f766572776f726c64a6506c61796572",
			),
		);
		expect(msg).toEqual({
			StateSnapshot: {
				tick: 3,
				last_processed_seq: 4,
				entities: [
					{
						id: 5,
						position: { x: 1, y: 2 },
						velocity: { x: 0, y: 0 },
						zone: { Overworld: null },
						entity_type: "Player",
					},
				],
			},
		});
	});
});

describe("normalizeServerMessage (legacy map-shaped payloads)", () => {
	it("accepts named JoinAck", () => {
		const msg = normalizeServerMessage({
			JoinAck: {
				player_id: "00010203-0405-0607-0809-0a0b0c0d0e0f",
				entity_id: 1,
				zone: { Overworld: null },
				world_seed: 42,
			},
		});
		expect("JoinAck" in msg).toBe(true);
		if (!("JoinAck" in msg)) throw new Error("expected JoinAck");
		expect(msg.JoinAck.player_id).toBe("00010203-0405-0607-0809-0a0b0c0d0e0f");
	});

	it("accepts named StateSnapshot entities", () => {
		const msg = normalizeServerMessage({
			StateSnapshot: {
				tick: 1,
				last_processed_seq: 2,
				entities: [
					{
						id: 9,
						position: { x: 1, y: 2 },
						velocity: { x: 0, y: 0 },
						zone: { AsteroidBase: 3 },
						entity_type: "Hostile",
					},
				],
			},
		});
		expect("StateSnapshot" in msg).toBe(true);
		if (!("StateSnapshot" in msg)) throw new Error("expected StateSnapshot");
		expect(msg.StateSnapshot.entities).toHaveLength(1);
		expect(msg.StateSnapshot.entities[0].zone).toEqual({ AsteroidBase: 3 });
	});

	it("normalizes structured inventory snapshot", () => {
		const msg = normalizeServerMessage({
			InventorySnapshot: {
				inventory: {
					entity_id: 10,
					items: [
						{
							item: "iron",
							quantity: 4,
							rarity: "Common",
							category: "Resource",
						},
					],
				},
			},
		});
		expect("InventorySnapshot" in msg).toBe(true);
		if (!("InventorySnapshot" in msg))
			throw new Error("expected InventorySnapshot");
		expect(msg.InventorySnapshot.inventory.items[0].item).toBe("iron");
	});

	it("normalizes trader quote with daily limits", () => {
		const msg = normalizeServerMessage({
			TraderQuotes: {
				quotes: [
					{
						trader_entity_id: 1,
						item: "iron",
						buy_price: 10,
						sell_price: 12,
						demand_scalar: 1,
						available_credits: 2500,
						daily_credit_limit: 5000,
						daily_credits_used: 100,
					},
				],
			},
		});
		expect("TraderQuotes" in msg).toBe(true);
		if (!("TraderQuotes" in msg)) throw new Error("expected TraderQuotes");
		expect(msg.TraderQuotes.quotes[0].daily_credit_limit).toBe(5000);
	});
});
