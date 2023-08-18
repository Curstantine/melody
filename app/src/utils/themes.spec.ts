import { expect, suite, test } from "vitest";
import { prepareUnoThemePaletteDefinitions } from "./themes";

suite("prepareUnoThemePaletteDefinitions", () => {
	test("should prepare a color palette useable by unocss", () => {
		const result = prepareUnoThemePaletteDefinitions();

		expect(result).toBeDefined();
		expect(result).toHaveProperty("border");
		expect(result).toHaveProperty("background");
		expect(result).toHaveProperty("titlebar");
	});
});
