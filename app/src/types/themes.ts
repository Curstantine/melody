export type ThemeColorBackground = "background.main" | "background.secondary";

export type ThemeColorButtonStateVariants = "hover" | "active" | "disabled";
export type ThemeColorButton = `background.${ThemeColorButtonStateVariants}`;

export type ThemeColorTitleBarButton = Exclude<ThemeColorButton, "background.disabled">;
export type ThemeColorTitleBar =
	| "titlebar.background"
	| `titlebar.button.close.${ThemeColorTitleBarButton}`
	| `titlebar.button.common.${ThemeColorTitleBarButton}`;

export type ThemeColorKeys =
	| ThemeColorBackground
	| ThemeColorTitleBar
	| "border";

export type ThemeColors = Record<ThemeColorKeys, string>;

export interface ThemeData {
	name: string;
	mode: "light" | "dark";
	colors: Partial<ThemeColors>;
}
