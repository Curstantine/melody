import { ThemeColorKeys, ThemeData } from "@/types/themes";

const templateTheme = "modern_dark";

export async function initialize() {
	let themeName = localStorage.getItem("theme");

	if (!themeName) {
		themeName = window.matchMedia("(prefers-color-scheme: dark)").matches ? templateTheme : templateTheme;
	}

	loadTheme(themeName);
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
	const { style } = document.documentElement;

	style.setProperty("--theme-name", data.name);
	style.setProperty("color-scheme", data.mode);

	for (const [key, value] of Object.entries(data.colors)) {
		style.setProperty(themeBindings[key as ThemeColorKeys], value);
	}

	// localStorage.setItem("theme", themeId);
}

const themeBindings: Record<ThemeColorKeys, string> = {
	"border": "--theme-border",
	"background.main": "--theme-background-main",
	"background.secondary": "--theme-background-secondary",

	"titlebar.background": "--theme-titlebar-background",
	"titlebar.button.close.active": "--theme-titlebar-button-close-active",
	"titlebar.button.close.hover": "--theme-titlebar-button-close-hover",
	"titlebar.button.close.disabled": "--theme-titlebar-button-close-disabled",
	"titlebar.button.common.active": "--theme-titlebar-button-common-active",
	"titlebar.button.common.hover": "--theme-titlebar-button-common-hover",
	"titlebar.button.common.disabled": "--theme-titlebar-button-common-disabled",
};
