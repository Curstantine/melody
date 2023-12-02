import type { BackendActionEntity, BackendActionPayload, BackendPathedError } from "@/types/backend";

export type LibraryCommand = "create_library" | "get_libraries";
export type LibraryEvent = "library_scan";

export type LibraryEntity = BackendActionEntity<Library>;
export type LibraryActionPayload = BackendActionPayload<LibraryAction, BackendPathedError>;

export interface Library {
	name: string;
	scanLocations: string[];
}

export interface LibraryAction {
	type: "reading" | "indexing";
	total: number;
	current: number;
	path: string;
}

export interface LibraryCreateParameters {
	[key: string]: unknown;
	name: string;
	scanLocations: string[];
}
