import { LocalError } from "@/types/errors";

export default class ThemeError implements LocalError {
	type: string = "ThemeError";
	code: number;
	message: string;
	context?: string;

	constructor(code: number, message: string, context?: string) {
		this.code = code;
		this.message = message;
		this.context = context;
	}

	static missingTheme(id: string): ThemeError {
		return new ThemeError(
			ThemeError.codes.MISSING_THEME,
			"Failed to load the theme!",
			`Couldn't find a theme with id: ${id}`,
		);
	}

	static codes = {
		MISSING_THEME: 1,
	};
}
