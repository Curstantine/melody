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
		shortcuts: {
			"button-layout": [
				"inline-flex items-center justify-center",
				"px-4 font-orbiter-deck rounded h-8 transition-colors duration-standard ease-standard",
			].join(" "),
			"icon-button-layout": [
				"inline-flex items-center justify-center",
				"rounded transition-colors duration-standard ease-standard",
			].join(" "),
			"button-error": [
				"bg-button-error-background-idle text-button-error-text-idle",
				"hover:(bg-button-error-background-hover text-button-error-text-hover)",
				"active:(bg-button-error-background-active text-button-error-text-active)",
			].join(" "),
			"button-primary": [
				"bg-button-primary-background-idle text-button-primary-text-idle",
				"hover:(bg-button-primary-background-hover text-button-primary-text-hover)",
				"active:(bg-button-primary-background-active text-button-primary-text-active)",
			].join(" "),
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
