import { createSignal } from "solid-js";

import type { AppError } from "@/types/errors";
import type { SignalObject } from "@/types/solid";

export default class AppModel {
	appError: SignalObject<AppError | null>;

	constructor() {
		const [appError, setAppError] = createSignal<AppError | null>(null);
		this.appError = { get: appError, set: setAppError };
	}
}
