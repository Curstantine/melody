import { resolve as resolvePath } from "node:path";
import UnoCSS from "unocss/vite";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

import authInfoPlugin from "./plugins/authInfo";

export default defineConfig({
	plugins: [UnoCSS(), solid(), authInfoPlugin()],
	resolve: {
		alias: [
			{
				find: "@",
				replacement: resolvePath("src"),
			},
		],
	},
});
