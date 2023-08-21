import type { IconifyJSON } from "@iconify/types";
import {
	Awaitable,
	defineConfig,
	presetIcons,
	presetUno,
	transformerDirectives,
	transformerVariantGroup,
} from "unocss";
import type { Theme } from "unocss/preset-uno";

import presetTheme from "./plugins/preset-theme";

const importIconCollection = (name: string): () => Awaitable<IconifyJSON> => {
	return async () => {
		const { default: icons } = await import(`@iconify-json/${name}/icons.json`);
		return icons;
	};
};

export default defineConfig<Theme>({
	presets: [
		presetUno({}),
		presetIcons({
			collections: {
				symbols: importIconCollection("material-symbols"),
				mdi: importIconCollection("mdi"),
			},
		}),
		presetTheme(),
	],
	transformers: [transformerDirectives(), transformerVariantGroup()],
	shortcuts: {
		"button-layout": [
			"inline-flex items-center justify-center",
			"px-4 font-orbiter-deck rounded h-8 transition-colors duration-300",
		].join(" "),
		"button-error": [
			"button-layout",
			"bg-button-error-background-idle text-button-error-text-idle",
			"hover:(bg-button-error-background-hover text-button-error-text-hover)",
			"active:(bg-button-error-background-active text-button-error-text-active)",
		].join(" "),
		"button-primary": [
			"button-layout",
			"bg-button-primary-background-idle text-button-primary-text-idle",
			"hover:(bg-button-primary-background-hover text-button-primary-text-hover)",
			"active:(bg-button-primary-background-active text-button-primary-text-active)",
		].join(" "),
	},
});
