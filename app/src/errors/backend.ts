import type { BackendBaseError } from "@/types/backend";
import { LocalError } from "@/types/errors";

export default class BackendError extends Error implements LocalError {
	name = "BackendError";
	message: string;
	context?: string;

	/**
	 * Backend errors doesn't support error codes.
	 * This will always be -1
	 */
	code = -1;

	constructor(short: string, message?: string) {
		super();
		this.message = short;
		this.context = message;
	}

	public static fromStupidError(error: unknown): BackendError {
		switch (typeof error) {
			case "string":
				return new BackendError("Unknown Backend Error", error);
			default: {
				const e = error as BackendBaseError;
				return new BackendError(e.short, e.message);
			}
		}
	}

	public static placeholder(): BackendError {
		return new BackendError(
			"Placeholder error",
			"Really long contextual message!! that spans across multiple lines!!!",
		);
	}

	public getMultilineContext(): string[] | null {
		if (!this.context) return null;
		return Array.isArray(this.context) ? this.context : this.context.split("\n");
	}
}
