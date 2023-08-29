import type { ThemeColorKeys, ThemeData } from "@/types/themes";

import ThemeError from "@/errors/themes";
import Result from "@/utils/result";
import { themeBindings } from "@/utils/themes.node";

const templateTheme = "dark";
const defaultThemeId = "dark";

export async function initialize(): Promise<Result<void, ThemeError>> {
	let themeId = localStorage.getItem("theme");
	if (!themeId) {
		themeId = window.matchMedia("(prefers-color-scheme: dark)").matches ? templateTheme : templateTheme;
	}

	return await Result.runAsync(
		async () => await loadTheme(themeId!),
		() => {
			loadTheme(defaultThemeId);
			return ThemeError.missingTheme(themeId!);
		},
	);
}

/**
 * Imports and load a theme from the `assets/theme` folder.
 *
 * @param {string} themeId Usually the name of the json file in the `assets/theme` folder.
 *
 * @throws Error if the theme name couldn't be imported by vite's dynamic import
 */
export async function loadTheme(themeId: string) {
	const data = await import(`../assets/themes/${themeId}.json`) as ThemeData;

	const styleElement = document.head.querySelector<HTMLStyleElement>("style#theme-declarations")!;
	const declarations = Object.entries(data.colors).map(([key, value]) =>
		`${themeBindings[key as ThemeColorKeys]}: ${value};`
	);

	// This allows us to atomically change every declaration, so animations and transitions wouldn't break.
	styleElement.innerHTML = `:root {
		--theme-name: ${data.name};
		color-scheme: ${data.mode};
		${declarations.join("\n")}
	}`;
}
