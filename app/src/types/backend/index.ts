import type { LibraryCommand, LibraryEvent } from "@/types/backend/library";

export type GeneralCommand = "setup";
export type BackendCommands = GeneralCommand | LibraryCommand;
export type BackendEvents = LibraryEvent;

export type BackendErrorTypes =
	| "io"
	| "descriptive"
	| "conversion"
	| "tokio"
	| "database"
	| "tauri"
	| "serde"
	| "symphonia";

export interface BackendBaseError {
	type: BackendErrorTypes;
	message: string;
	context?: string;
}
