import js from "@eslint/js"
import globals from "globals"

export default [
	{
		...js.configs.recommended,
		files: ["src/**/*.js"],
		ignores: ["src/**/*.min.js"],
		languageOptions: {
			ecmaVersion: "latest",
			sourceType: "module",
			globals: {
				...globals.browser
			}
		}
	}
]
