import type { BackendBaseError, BackendErrorTypes } from "@/types/backend";
import { LocalError } from "@/types/errors";

export default class BackendError extends Error implements LocalError {
	name = "BackendError";
	message: string;

	type: BackendErrorTypes;
	context?: string | string[];

	/**
	 * Backend errors doesn't support error codes.
	 * This will always be -1
	 */
	code = -1;

	constructor(type: BackendErrorTypes, message: string, context?: string | string[]) {
		super();
		this.type = type;
		this.message = message;
		this.context = context;
	}

	public static fromStupidError(error: unknown): BackendError {
		// TODO: Handle structural integrity errors

		switch (typeof error) {
			case "string":
				return new BackendError("descriptive", "Possibly a tauri error", error);
			default: {
				const e = error as BackendBaseError;
				return new BackendError(e.type, e.message, e.context);
			}
		}
	}

	public static placeholder(): BackendError {
		return new BackendError(
			"io",
			"Placeholder error",
			["Really long contextual message!!", "that spans across multiple lines!!!"],
		);
	}

	public getMultilineContext(): string[] | null {
		if (!this.context) return null;
		return Array.isArray(this.context) ? this.context : this.context.split("\n");
	}
}
