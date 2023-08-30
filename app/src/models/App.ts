import { createSignal } from "solid-js";

import type { AppError } from "@/types/errors";

import LibraryManager from "@/models/LibraryManager";

export default class AppModel {
	appError = createSignal<AppError | null>(null);
	libraryManager = new LibraryManager();
}
