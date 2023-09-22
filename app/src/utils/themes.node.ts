/*
 * This file should be shareable between the vite build, and local node scripts.
 * Do not use references to path aliases in this file.
 */

import type { ThemeColorKeys } from "../types/themes";

export const durations = {
	standard: 300,
	emphasized: 500,
};

export const easingFunctions = {
	emphasized: "cubic-bezier(0.4, 0.0, 0.2, 1.0)",
	standard: "cubic-bezier(0.2, 0.0, 0, 1.0)",
};

export const themeBindings: Record<ThemeColorKeys, string> = {
	"border.main": "--theme-border-main",
	"border.secondary": "--theme-border-secondary",
	"background.main": "--theme-background-main",
	"background.secondary": "--theme-background-secondary",

	"text.1": "--theme-text-1",
	"text.2": "--theme-text-2",
	"text.3": "--theme-text-3",
	"text.error": "--theme-text-error",

	"titlebar.background": "--theme-titlebar-background",
	"titlebar.button.close.background.active": "--theme-titlebar-button-close-active",
	"titlebar.button.close.background.hover": "--theme-titlebar-button-close-hover",
	"titlebar.button.common.background.active": "--theme-titlebar-button-common-active",
	"titlebar.button.common.background.hover": "--theme-titlebar-button-common-hover",

	"modal.error.border": "--theme-modal-error-border",
	"modal.error.text.primary": "--theme-modal-error-text-primary",

	"button.primary.background.idle": "--theme-button-primary-idle",
	"button.primary.background.hover": "--theme-button-primary-hover",
	"button.primary.background.active": "--theme-button-primary-active",
	"button.primary.text.idle": "--theme-button-primary-text-idle",
	"button.primary.text.hover": "--theme-button-primary-text-hover",
	"button.primary.text.active": "--theme-button-primary-text-active",

	"button.error.background.idle": "--theme-button-error-idle",
	"button.error.background.hover": "--theme-button-error-hover",
	"button.error.background.active": "--theme-button-error-active",
	"button.error.text.idle": "--theme-button-error-text-idle",
	"button.error.text.hover": "--theme-button-error-text-hover",
	"button.error.text.active": "--theme-button-error-text-active",

	"input.border.idle": "--theme-input-border-idle",
	"input.border.active": "--theme-input-border-active",

	"loader.background": "--theme-loader-background",
	"loader.foreground": "--theme-loader-foreground",
};
