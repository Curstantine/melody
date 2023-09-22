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

	return mergeDeep({}, obj) as Theme["colors"];
}
