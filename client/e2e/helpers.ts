import net from "node:net";
import { expect, type Page } from "@playwright/test";

export async function isPortOpen(
	host: string,
	port: number,
	timeoutMs = 800,
): Promise<boolean> {
	return new Promise((resolve) => {
		const socket = new net.Socket();
		let settled = false;

		const finalize = (open: boolean) => {
			if (settled) return;
			settled = true;
			socket.destroy();
			resolve(open);
		};

		socket.setTimeout(timeoutMs);
		socket.once("connect", () => finalize(true));
		socket.once("error", () => finalize(false));
		socket.once("timeout", () => finalize(false));
		socket.connect(port, host);
	});
}

export async function currentSceneKey(page: Page): Promise<string | null> {
	return page.evaluate(() => {
		const hostWindow = window as Window & {
			__PHASER_GAME__?: {
				scene: {
					getScenes(activeOnly?: boolean): Array<{ scene: { key: string } }>;
				};
			};
		};
		const game = hostWindow.__PHASER_GAME__;
		if (!game) return null;
		const activeScenes = game.scene.getScenes(true);
		if (activeScenes.length === 0) return null;
		return activeScenes[0]?.scene.key ?? null;
	});
}

export async function expectScene(page: Page, expected: string): Promise<void> {
	await expect
		.poll(async () => currentSceneKey(page), {
			timeout: 20_000,
			message: `expected active scene ${expected}`,
		})
		.toBe(expected);
}
