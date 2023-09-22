import { type Preset, presetIcons, presetUno } from "unocss";
import type { Theme } from "unocss/preset-uno";
import { mergeDeep } from "utils";

import { getColorDefinitions } from "./colors";
import { buttonShortcuts } from "./components";
import { iconCollection } from "./icons";
import { prepareUnoFontDefinitions } from "./text";

export default function presetTheme(): Preset<Theme> {
	return {
		name: "preset-theme-unocss",
		presets: [
			presetUno({}),
			presetTheme(),
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
		shortcuts: mergeDeep({}, buttonShortcuts),
	};
}
