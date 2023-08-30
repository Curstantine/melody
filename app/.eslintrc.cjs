const { defineConfig } = require("eslint-define-config");

module.exports = defineConfig({
	extends: ["../.eslintrc.cjs", "@unocss"],
	plugins: ["solid"],
	rules: {
		"@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
	},
});
