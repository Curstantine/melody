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
	theme: {
		easing: {
			DEFAULT: "cubic-bezier(0.4, 0.0, 0.2, 1.0)",
			standard: "cubic-bezier(0.2, 0.0, 0, 1.0)",
			"standard-decelerate": "cubic-bezier(0, 0, 0, 1)",
			"standard-accelerate": "cubic-bezier(0.3, 0, 1, 1)",
		},
	},
});
