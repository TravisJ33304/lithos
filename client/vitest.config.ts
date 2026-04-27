import { defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		typecheck: {
			enabled: false,
		},
		include: ["src/**/*.test.ts"],
	},
});
