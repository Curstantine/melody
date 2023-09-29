export type GeneralCommand = "setup";

export type LibraryCommand = "create_library" | "get_library_names";
export type LibraryEvent = "library_scan";

export type BackendCommands = GeneralCommand | LibraryCommand;
export type BackendEvents = LibraryEvent;

export interface LibraryGenericActionPayload {
	action_type: "reading" | "indexing";
	total: number;
	current: number;
	path: string;
}

export interface LibraryCreateParameters {
	name: string;
	scanLocations: string[];
}
