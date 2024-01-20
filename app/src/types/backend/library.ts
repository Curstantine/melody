import type { BackendEventPayload, BackendPathedError } from "@/types/backend";

export type LibraryCommand = "get_scan_locations" | "initialize_library";
export type LibraryEventType = "scan";

export type LibraryEventPayload = BackendEventPayload<LibraryEvent, BackendPathedError>;

export interface LibraryEvent {
	type: "indexing" | "scanning";
	data: string | LibraryEventData;
}

export interface LibraryEventData {
	total: number;
	current: number;
	path: string;
}

export interface LibraryCreateParameters {
	[key: string]: unknown;
	scanLocations: string[];
}
