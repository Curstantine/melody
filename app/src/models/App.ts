import { createSignal } from "solid-js";

import type { AppError } from "@/types/errors";
import type { SignalObject } from "@/types/solid";

import LibraryManager from "@/models/LibraryManager";

export default class AppModel {
	appError: SignalObject<AppError | null>;
	libraryManager: LibraryManager;

	constructor() {
		this.libraryManager = new LibraryManager();

		const [appError, setAppError] = createSignal<AppError | null>(null);
		this.appError = { get: appError, set: setAppError };
	}
}
