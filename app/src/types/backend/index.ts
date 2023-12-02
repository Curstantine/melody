import type { LibraryCommand, LibraryEventType } from "@/types/backend/library";
import type { ReleaseCommand } from "@/types/backend/release";

export type GeneralCommand = "setup";
export type BackendCommands = GeneralCommand | LibraryCommand | ReleaseCommand;
export type BackendEvents = LibraryEventType;

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

export interface BackendPathedError {
	path: string;
	error: BackendBaseError;
}

export interface BackendEventPayload<T, E> {
	type: "ok" | "error";
	data: T | E;
}

export interface BackendEntity<T> {
	id: number;
	attributes: T;
}
