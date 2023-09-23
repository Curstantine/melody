import { type Preset, presetIcons, presetUno } from "unocss";
import type { Theme } from "unocss/preset-uno";

import { getColorDefinitions } from "./colors.js";
import { buttonShortcuts } from "./components.js";
import { iconCollection } from "./icons.js";
import { prepareUnoFontDefinitions } from "./text.js";

export default function presetTheme(): Preset<Theme> {
	return {
		name: "preset-theme-unocss",
		presets: [
			presetUno({}),
			presetIcons({
				collections: iconCollection,
			}),
		],
		theme: {
			colors: getColorDefinitions(),
			fontFamily: prepareUnoFontDefinitions(),
			easing: {
				emphasized: "cubic-bezier(0.4, 0.0, 0.2, 1.0)",
				standard: "cubic-bezier(0.2, 0.0, 0, 1.0)",
			},
			duration: {
				standard: "300ms",
				emphasized: "500ms",
			},
		},
		shortcuts: buttonShortcuts,
	};
}
