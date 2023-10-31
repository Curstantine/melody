import { BackendBaseError } from "@/types/backend";

export type LibraryCommand = "create_library" | "get_library_names";
export type LibraryEvent = "library_scan";

export interface LibraryActionPayload {
	type: "ok" | "error";
	data: LibraryActionData | LibraryActionError;
}

export interface LibraryActionError {
	error: BackendBaseError;
	path: string;
}

export interface LibraryActionData {
	action_type: "reading" | "indexing";
	total: number;
	current: number;
	path: string;
}

export interface LibraryCreateParameters {
	[key: string]: unknown;
	name: string;
	scanLocations: string[];
}
