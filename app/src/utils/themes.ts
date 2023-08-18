import type { ThemeColorKeys, ThemeData } from "@/types/themes";
import { Theme } from "unocss/preset-uno";
import { createNestedPropertyValue, mergeDeep } from "./general";

const templateTheme = "modern_dark";

export async function initialize() {
	let themeName = localStorage.getItem("theme");

	if (!themeName) {
		themeName = window.matchMedia("(prefers-color-scheme: dark)").matches ? templateTheme : templateTheme;
	}

	await loadTheme(themeName);
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

	// localStorage.setItem("theme", themeId);
}

/**
 * Should only really be used as part of unocss preset plugin to add definitions for the extensible themes.
 */
export function prepareUnoThemePaletteDefinitions(): Theme["colors"] {
	const obj: Record<string, Record<string, unknown>> = {};

	for (const [key, value] of Object.entries(themeBindings)) {
		const splits = key.split(".");
		obj[key] = createNestedPropertyValue(splits, `var(${value})`);
	}

	return mergeDeep({}, ...Object.values(obj)) as Theme["colors"];
}

export const themeBindings: Record<ThemeColorKeys, string> = {
	"border": "--theme-border",
	"background.main": "--theme-background-main",
	"background.secondary": "--theme-background-secondary",

	"titlebar.background": "--theme-titlebar-background",
	"titlebar.button.close.background.active": "--theme-titlebar-button-close-active",
	"titlebar.button.close.background.hover": "--theme-titlebar-button-close-hover",
	"titlebar.button.common.background.active": "--theme-titlebar-button-common-active",
	"titlebar.button.common.background.hover": "--theme-titlebar-button-common-hover",
};
