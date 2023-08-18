/// <reference types="vitest" />

import { resolve as resolvePath } from "node:path";
import UnoCSS from "unocss/vite";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

export default defineConfig({
	plugins: [UnoCSS(), solid()],
	resolve: {
		alias: [
			{
				find: "@",
				replacement: resolvePath("src"),
			},
		],
	},
	test: {
		environment: "jsdom",
		globals: true,
		deps: {
			optimizer: {
				web: {
					include: ["src/**/*.ts", "src/**/*.tsx"],
				},
			},
		},
	},
});
