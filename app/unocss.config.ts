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
		presetTheme(),
		presetIcons({
			collections: {
				symbols: importIconCollection("material-symbols"),
				mdi: importIconCollection("mdi"),
			},
		}),
	],
	transformers: [transformerDirectives(), transformerVariantGroup()],
});
