import js from "@eslint/js";
import solid from "eslint-plugin-solid/configs/typescript";
import * as tsParser from "@typescript-eslint/parser";
import unocss from "@unocss/eslint-config/flat";

export default [
    unocss,
    js.configs.recommended,
    {
        files: ["**/*.{ts,tsx}"],
        ...solid,
        languageOptions: {
            parser: tsParser,
            parserOptions: {
                project: "tsconfig.json",
            },
        },
    },
];
