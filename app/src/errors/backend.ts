import { LocalError } from "@/types/errors";

type BackendErrorTypes =
	| "std_io"
	| "std_parse_int"
	| "chrono_parse"
	| "descriptive"
	| "conversion"
	| "tokio_task"
	| "tauri"
	| "bonsai_local"
	| "bonsai_core"
	| "serde";

export default class BackendError implements LocalError {
	type: BackendErrorTypes;
	code: number;
	message: string;
	context?: string | string[];

	constructor(type: BackendErrorTypes, code: number, message: string, context?: string | string[]) {
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
