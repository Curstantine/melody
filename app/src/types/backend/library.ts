import type { BackendEntity, BackendEventPayload, BackendPathedError } from "@/types/backend";

export type LibraryCommand = "create_library" | "get_libraries";
export type LibraryEventType = "scan";

export type LibraryEntity = BackendEntity<Library>;
export type LibraryEventPayload = BackendEventPayload<LibraryEvent, BackendPathedError>;

export interface Library {
	name: string;
	scanLocations: string[];
}

export interface LibraryEvent {
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
