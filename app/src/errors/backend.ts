import type { BackendBaseError, BackendErrorTypes } from "@/types/backend";
import { LocalError } from "@/types/errors";

export default class BackendError implements LocalError {
	type: BackendErrorTypes;
	message: string;
	context?: string | string[];

	/**
	 * Backend errors doesn't support error codes.
	 * This will always be -1
	 */
	code = -1;

	constructor(type: BackendErrorTypes, message: string, context?: string | string[]) {
		this.type = type;
		this.message = message;
		this.context = context;
	}

	public static fromStupidError(error: unknown): BackendError {
		// TODO: Handle structural integrity errors

		const e = error as BackendBaseError;
		return new BackendError(e.type, e.message, e.context);
	}
}
