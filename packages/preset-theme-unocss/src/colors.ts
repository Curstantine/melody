import type { Theme } from "unocss/preset-uno";
import { createNestedPropertyValue, mergeDeep } from "utils";

import { properties as definition } from "../../../resources/schemas/theme.json";

const colorDefinitions = definition.colors.properties;

export function getColorDefinitions(): Theme["colors"] {
	const obj: Record<string, Record<string, unknown>> = {};
	const properties = Object.keys(colorDefinitions);

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
	const declarations = Object.entries(theme.colors).map(([key, value]) => `--${key.replaceAll(".", "-")}: ${value};`);

	return `:root {
		--theme-name: ${theme.name};
		color-scheme: ${theme.mode};
		${declarations.join("\n")}
	}`;
}
