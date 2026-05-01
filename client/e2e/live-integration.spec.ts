import { expect, test } from "@playwright/test";
import { expectScene, isPortOpen } from "./helpers";

let liveServerAvailable = false;
const liveWsUrl = process.env.LITHOS_E2E_WS_URL ?? "ws://localhost:9001";
const liveServerPort = Number(process.env.LITHOS_E2E_SERVER_PORT ?? "9001");

test.beforeAll(async () => {
	liveServerAvailable =
		process.env.LITHOS_RUN_LIVE_E2E === "1" &&
		(await isPortOpen("127.0.0.1", liveServerPort));
});

test("joins local server and exercises core in-game flow", async ({ page }) => {
	test.skip(
		!liveServerAvailable,
		"set LITHOS_RUN_LIVE_E2E=1 with lithos-server running at LITHOS_E2E_WS_URL",
	);

	const consoleErrors: string[] = [];
	page.on("console", (message) => {
		if (message.type() === "error") {
			consoleErrors.push(message.text());
		}
	});

	await page.route("http://localhost:3000/v1/servers", async (route) => {
		await route.fulfill({
			status: 200,
			contentType: "application/json",
			body: JSON.stringify([
				{
					server_id: "local-shard",
					name: "Local Dev Shard",
					region: "local",
					websocket_url: liveWsUrl,
					population: 1,
					capacity: 100,
					healthy: true,
				},
			]),
		});
	});

	await page.goto("/");
	await page.click(".ui-server-row");
	await page.fill("#ui-username", "playwright#1");
	await page.click("#ui-join-btn");

	await expectScene(page, "LoginScene");
	await page.click("#ui-login-btn");

	await expectScene(page, "OverworldScene");
	await expect(page.locator("#ui-hud")).toBeVisible();
	await expect(page.locator("#ui-inventory")).toBeVisible();
	await expect(page.locator("#ui-crafting")).toBeVisible();
	await expect(page.locator("#ui-onboarding")).toBeVisible();
	await expect(page.locator("#ui-crafting-summary")).not.toHaveText(
		"0 items | 0 recipes",
	);

	await page.fill("#ui-chat-input", "playwright e2e chat");
	await page.press("#ui-chat-input", "Enter");
	await expect(page.locator("#ui-chat-log")).toContainText(
		"playwright e2e chat",
	);
	await page.locator("#ui-chat-input").evaluate((el) => {
		(el as HTMLInputElement).blur();
	});

	await page.keyboard.down("Space");
	await page.waitForTimeout(100);
	await page.keyboard.up("Space");
	await expectScene(page, "AsteroidBaseScene");

	await page.keyboard.down("Space");
	await page.waitForTimeout(100);
	await page.keyboard.up("Space");
	await expectScene(page, "OverworldScene");

	expect(consoleErrors).toEqual([]);
});
