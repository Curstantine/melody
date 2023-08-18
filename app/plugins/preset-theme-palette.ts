import type { Preset } from "unocss";
import type { Theme } from "unocss/preset-uno";

import { createNestedPropertyValue, mergeDeep } from "../src/utils/general.node";
import { themeBindings } from "../src/utils/themes.node";

export default function presetThemePalette(): Preset<Theme> {
	return {
		name: "theme-palette",
		theme: {
			colors: prepareUnoThemePaletteDefinitions(),
		},
	};
}

function prepareUnoThemePaletteDefinitions(): Theme["colors"] {
	const obj: Record<string, Record<string, unknown>> = {};

	for (const [key, value] of Object.entries(themeBindings)) {
		const splits = key.split(".");
		obj[key] = createNestedPropertyValue(splits, `var(${value})`);
	}

	return mergeDeep({}, ...Object.values(obj)) as Theme["colors"];
}
