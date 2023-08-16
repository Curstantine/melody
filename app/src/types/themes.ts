export type ThemeColorBackground = "background.main" | "background.secondary";

export type ThemeColorButtonStateVariants = "hover" | "active" | "disabled";

export type ThemeColorTitleBar =
	| "titlebar.background"
	| `titlebar.button.close.${ThemeColorButtonStateVariants}`
	| `titlebar.button.common.${ThemeColorButtonStateVariants}`;

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
