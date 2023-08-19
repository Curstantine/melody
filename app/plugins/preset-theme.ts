import type { Preset } from "unocss";
import type { Theme } from "unocss/preset-uno";

import { createNestedPropertyValue, mergeDeep } from "../src/utils/general.node";
import { durations, easingFunctions, themeBindings } from "../src/utils/themes.node";

export default function presetTheme(): Preset<Theme> {
	return {
		name: "theme",
		theme: {
			colors: prepareUnoColorDefinitions(),
			fontFamily: prepareUnoFontDefinitions(),
			easing: easingFunctions,
			duration: {
				emphasized: `${durations.emphasized}ms`,
				standard: `${durations.standard}ms`,
			},
		},
	};
}

function prepareUnoColorDefinitions(): Theme["colors"] {
	const obj: Record<string, Record<string, unknown>> = {};

	for (const [key, value] of Object.entries(themeBindings)) {
		const splits = key.split(".");
		obj[key] = createNestedPropertyValue(splits, `var(${value})`);
	}

	return mergeDeep({}, ...Object.values(obj)) as Theme["colors"];
}

function prepareUnoFontDefinitions(): Theme["fontFamily"] {
	const explorer = createFontVariants("TASA Explorer", "explorer");
	const orbiter = {
		deck: createFontVariants("TASA Orbiter Deck", "orbiter-deck"),
		display: createFontVariants("TASA Orbiter Display", "orbiter-display"),
		text: createFontVariants("TASA Orbiter Text", "orbiter-text"),
	};

	return mergeDeep({}, explorer, ...Object.values(orbiter)) as Theme["fontFamily"];
}

const variantToFontStyleBind = {
	regular: "Regular",
	medium: "Medium",
	semi: "Semibold",
	bold: "Bold",
};

const createFontVariants = (
	name: string,
	prefix: string,
	variants = ["regular", "medium", "semi", "bold"],
) => {
	const obj: Record<string, string> = {};

	for (const variant of variants) {
		const key = `${prefix}${variant === "regular" ? "" : "-" + variant}`;
		obj[key] = `"${name} ${variantToFontStyleBind[variant]}", system-ui`;
	}

	return obj;
};
