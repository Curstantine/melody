export function isObject(item: unknown): item is Record<string, unknown> {
	return !!(item && typeof item === "object" && !Array.isArray(item));
}

export function mergeDeep(target: Record<string, unknown>, ...sources: Record<string, unknown>[]) {
	if (!sources.length) return target;
	const source = sources.shift();

	if (isObject(target) && isObject(source)) {
		for (const key in source) {
			if (isObject(source[key])) {
				if (!target[key]) Object.assign(target, { [key]: {} });
				mergeDeep(target[key] as Record<string, unknown>, source[key] as Record<string, unknown>);
			} else {
				Object.assign(target, { [key]: source[key] });
			}
		}
	}

	return mergeDeep(target, ...sources);
}

/**
 * Creates an object that nests a value under the given properties.
 *
 * E.g.:
 * ```ts
 * const nestedObject = createNestedObject(["a", "b", "c"], 1);
 * console.log(nestedObject); // { a: { b: { c: 1 } } }
 * ```
 *
 * @param properties The names of the properties {@link value} should be nested under.
 * @param value The value that needs to be assigned the last {@link properties}
 */
export function createNestedPropertyValue(properties: string[], value: unknown) {
	const nestedObject: Record<string, unknown> = {};
	let currentObject: Record<string, unknown> = nestedObject;

	for (let i = 0; i < properties.length; i++) {
		const property = properties[i];

		currentObject[property] = {};
		if (i + 1 < properties.length) currentObject = currentObject[property] as Record<string, unknown>;
		else currentObject[property] = value;
	}

	return nestedObject;
}
