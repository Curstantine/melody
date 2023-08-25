import { LocalError } from "@/types/errors";

export default class LibraryManagerError implements LocalError {
	type: string = "LibraryManagerError";
	code: number;
	message: string;
	context?: string;

	constructor(code: number, message: string, context?: string) {
		this.code = code;
		this.message = message;
		this.context = context;
	}

	static unknownLibrary(id: string): LibraryManagerError {
		return new LibraryManagerError(
			LibraryManagerError.codes.UNKNOWN_LIBRARY,
			"Failed to load the library!",
			`Couldn't find a library with id: ${id}`,
		);
	}

	static codes = {
		UNKNOWN_LIBRARY: 1,
	};
}
