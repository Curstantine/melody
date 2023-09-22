import type { Theme } from "unocss/preset-uno";
import { mergeDeep } from "utils";

export type FontVariants = "regular" | "medium" | "semi" | "bold";

export function prepareUnoFontDefinitions(): Theme["fontFamily"] {
	const orbiter = {
		deck: createFontVariants("TASA Orbiter Deck", "orbiter-deck"),
		display: createFontVariants("TASA Orbiter Display", "orbiter-display"),
		text: createFontVariants("TASA Orbiter Text", "orbiter-text"),
	};

	return mergeDeep({}, orbiter) as Theme["fontFamily"];
}

function createFontVariants(
	name: string,
	prefix: string,
	variants: FontVariants[] = ["regular", "medium", "semi", "bold"],
) {
	const obj: Record<string, string> = {};

	for (const variant of variants) {
		const key = `${prefix}${variant === "regular" ? "" : "-" + variant}`;
		obj[key] = `"${name} ${variantToFontStyleBind[variant]}", system-ui`;
	}

	return obj;
}

const variantToFontStyleBind: Record<FontVariants, string> = {
	regular: "Regular",
	medium: "Medium",
	semi: "Semibold",
	bold: "Bold",
};
