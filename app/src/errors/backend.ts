import { LocalError } from "@/types/errors";

type BackendErrorTypes =
	| "io"
	| "tokio_task"
	| "tauri"
	| "descriptive"
	| "database"
	| "serde"
	| "parse_int"
	| "chrono_parse";

export default class BackendError implements LocalError {
	type: BackendErrorTypes;
	code: number;
	message: string;
	context?: string;

	constructor(type: BackendErrorTypes, code: number, message: string, context?: string) {
		this.type = type;
		this.code = code;
		this.message = message;
		this.context = context;
	}

	public static fromStupidError(error: unknown): BackendError {
		// TODO: Handle structural integrity errors

		const e = error as LocalError;
		return new BackendError(
			e.type as BackendErrorTypes,
			e.code,
			e.message,
			e.context,
		);
	}
}