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
});
