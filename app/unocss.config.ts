import type { IconifyJSON } from "@iconify/types";
import {
	Awaitable,
	defineConfig,
	Preset,
	presetIcons,
	presetUno,
	transformerDirectives,
	transformerVariantGroup,
} from "unocss";
import type { Theme } from "unocss/preset-uno";

import { prepareUnoThemePaletteDefinitions } from "./src/utils/themes";

const importIconCollection = (name: string): () => Awaitable<IconifyJSON> => {
	return async () => {
		const { default: icons } = await import(`@iconify-json/${name}/icons.json`);
		return icons;
	};
};

const presetThemePalette = (): Preset<Theme> => {
	return {
		name: "theme-palette",
		theme: {
			colors: prepareUnoThemePaletteDefinitions(),
		},
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
		presetThemePalette(),
	],
	transformers: [transformerDirectives(), transformerVariantGroup()],
	theme: {
		fontFamily: {
			"explorer": "TASA Explorer Regular, system-ui",
			"explorer-medium": "TASA Explorer Medium, system-ui",
			"explorer-semi": "TASA Explorer Semibold, system-ui",
			"explorer-bold": "TASA Explorer Bold, system-ui",

			"orbiter-deck": "TASA Orbiter Deck Regular, system-ui",
			"orbiter-deck-medium": "TASA Orbiter Deck Medium, system-ui",
			"orbiter-deck-semi": "TASA Orbiter Deck Semibold, system-ui",
			"orbiter-deck-bold": "TASA Orbiter Deck Bold, system-ui",

			"orbiter-display": "TASA Orbiter Display Regular, system-ui",
			"orbiter-display-medium": "TASA Orbiter Display Medium, system-ui",
			"orbiter-display-semi": "TASA Orbiter Display Semibold, system-ui",
			"orbiter-display-bold": "TASA Orbiter Display Bold, system-ui",

			"orbiter-text": "TASA Orbiter Text Regular, system-ui",
			"orbiter-text-medium": "TASA Orbiter Text Medium, system-ui",
			"orbiter-text-semi": "TASA Orbiter Text Semibold, system-ui",
			"orbiter-text-bold": "TASA Orbiter Text Bold, system-ui",
		},
		easing: {
			DEFAULT: "cubic-bezier(0.4, 0.0, 0.2, 1.0)",
			standard: "cubic-bezier(0.2, 0.0, 0, 1.0)",
			"standard-decelerate": "cubic-bezier(0, 0, 0, 1)",
			"standard-accelerate": "cubic-bezier(0.3, 0, 1, 1)",
		},
	},
});
