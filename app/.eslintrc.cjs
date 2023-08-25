module.exports = {
	extends: ["../.eslintrc.js", "@unocss"],
	plugins: ["solid"],
	rules: {
		"@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
	},
};
