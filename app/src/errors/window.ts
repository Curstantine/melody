import type { LocalError } from "@/types/errors";
import { getErrorFromUnknown } from "@/utils/exceptions";

export default class WindowError extends Error  implements LocalError {
	code: number;
	message: string;
	context?: string | string[];
	inner?: Error;

	constructor(code: number, message: string, context?: string | string[], inner?: Error) {
		super();
		this.code = code;
		this.message = message;
		this.context = context;
		this.inner = inner;
	}

	public static maximizeState(e: unknown): WindowError {
		const error = getErrorFromUnknown(e);

		return new WindowError(
			WindowError.codes.MAX_STATE,
			"Couldn't maximize the window",
			["Caller returned:", error?.message ?? "<NULL>"],
			error ?? undefined,
		);
	}

	public static focusState(e: unknown): WindowError {
		const error = getErrorFromUnknown(e);

		return new WindowError(
			WindowError.codes.MAX_STATE,
			"Couldn't focus the window",
			["Caller returned:", error?.message ?? "<NULL>"],
			error ?? undefined,
		);
	}

	static codes = {
		MAX_STATE: 1,
		FOCUS_STATE: 2,
	};
}
