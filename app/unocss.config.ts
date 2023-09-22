import presetTheme from "preset-theme-unocss";
import { defineConfig, transformerDirectives, transformerVariantGroup } from "unocss";
import type { Theme } from "unocss/preset-uno";

export default defineConfig<Theme>({
	presets: [presetTheme()],
	transformers: [transformerDirectives(), transformerVariantGroup()],
});
