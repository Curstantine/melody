export type ThemeColorBorder = "border.main" | "border.secondary";
export type ThemeColorBackground = "background.main" | "background.secondary";
export type ThemeColorText = "text.1" | "text.2" | "text.3" | "text.error";

export type ThemeColorButtonStateVariants = "idle" | "hover" | "active";
export type ThemeColorButtonComponent<S extends ThemeColorButtonStateVariants = ThemeColorButtonStateVariants> =
	| `background.${S}`
	| `text.${S}`;

export type ThemeColorTitleBarButton = `background.${Exclude<ThemeColorButtonStateVariants, "idle">}`;
export type ThemeColorTitleBar =
	| "titlebar.background"
	| `titlebar.button.close.${ThemeColorTitleBarButton}`
	| `titlebar.button.common.${ThemeColorTitleBarButton}`;

export type ThemeColorModal =
	| "modal.error.border"
	| "modal.error.text.primary";

export type ThemeColorButton = `button.error.${ThemeColorButtonComponent}`;

export type ThemeColorInputStateVariants = "idle" | "active";
export type ThemeColorInput = `input.border.${ThemeColorInputStateVariants}`;

export type ThemeColorKeys =
	| ThemeColorBorder
	| ThemeColorBackground
	| ThemeColorText
	| ThemeColorTitleBar
	| ThemeColorModal
	| ThemeColorButton
	| ThemeColorInput;

export type ThemeColors = Record<ThemeColorKeys, string>;

export interface ThemeData {
	name: string;
	mode: "light" | "dark";
	colors: Partial<ThemeColors>;
}
