/*
 * This file should be shareable between the vite build, and local node scripts.
 * Do not use references to path aliases in this file.
 */

import type { ThemeColorKeys } from "../types/themes";

export const themeBindings: Record<ThemeColorKeys, string> = {
	"border": "--theme-border",
	"background.main": "--theme-background-main",
	"background.secondary": "--theme-background-secondary",

	"text.1": "--theme-text-l1",
	"text.2": "--theme-text-l2",

	"titlebar.background": "--theme-titlebar-background",
	"titlebar.button.close.background.active": "--theme-titlebar-button-close-active",
	"titlebar.button.close.background.hover": "--theme-titlebar-button-close-hover",
	"titlebar.button.common.background.active": "--theme-titlebar-button-common-active",
	"titlebar.button.common.background.hover": "--theme-titlebar-button-common-hover",

	"modal.error.border": "--theme-modal-error-border",
	"modal.error.text.primary": "--theme-modal-error-text-primary",

	"button.error.background.idle": "--theme-button-error-idle",
	"button.error.background.hover": "--theme-button-error-hover",
	"button.error.background.active": "--theme-button-error-active",
	"button.error.text.idle": "--theme-button-error-text-idle",
	"button.error.text.hover": "--theme-button-error-text-hover",
	"button.error.text.active": "--theme-button-error-text-active",
};
