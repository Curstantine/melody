import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import authInfoPlugin from "./plugins/authInfo";

export default defineConfig({
	plugins: [solid(), authInfoPlugin()],
});
