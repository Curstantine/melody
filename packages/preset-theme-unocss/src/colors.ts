import type { Theme } from "unocss/preset-uno";
import { createNestedPropertyValue, mergeDeep } from "utils";

import themeSchema from "../../../resources/schemas/theme.json";

export function getColorDefinitions(): Theme["colors"] {
	const obj: Record<string, Record<string, unknown>> = {};
	const properties = Object.keys(themeSchema.properties.colors.properties);

	for (const key of properties) {
		const splits = key.split(".");
		const cssVariable = splits.join("-");

		obj[key] = createNestedPropertyValue(splits, `var(--${cssVariable})`);
	}

	return mergeDeep({}, ...Object.values(obj)) as Theme["colors"];
}

export interface ThemeConfig {
	name: string;
	mode: "dark" | "light";
	colors: Record<string, string>;
}

export function getCSSDefinitions(theme: ThemeConfig): string {
	const declarations = Object.entries(theme.colors).map(([key, value]) => `--${key.replace(".", "-")}: ${value};`);

	return `:root {
		--theme-name: ${theme.name};
		color-scheme: ${theme.mode};
		${declarations.join("\n")}
	}`;
}
