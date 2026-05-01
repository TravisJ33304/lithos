import { expect, test } from "@playwright/test";
import { expectScene } from "./helpers";

test("loads menu UI and transitions Boot -> Login via join", async ({
	page,
}) => {
	await page.route("http://localhost:3000/v1/servers", async (route) => {
		await route.fulfill({
			status: 200,
			contentType: "application/json",
			body: JSON.stringify([
				{
					server_id: "local-shard",
					name: "Local Dev Shard",
					region: "local",
					websocket_url: "ws://localhost:9001",
					population: 3,
					capacity: 100,
					healthy: true,
				},
			]),
		});
	});

	await page.goto("/");

	await expect(page.locator("#lithos-ui-root")).toBeVisible();
	await expect(page.locator("#ui-menu")).toBeVisible();
	await expect(page.locator(".ui-server-row")).toHaveCount(1);

	await page.click(".ui-server-row");
	await expect(page.locator("#ui-endpoint")).toHaveValue("ws://localhost:9001");

	await page.fill("#ui-username", "playwright-user");
	await page.click("#ui-join-btn");

	await expectScene(page, "LoginScene");
	await expect(page.locator("#username")).toHaveValue("playwright-user");
});
