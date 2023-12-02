import type { LibraryCommand, LibraryEvent } from "@/types/backend/library";
import type { ReleaseCommand } from "@/types/backend/release";

export type GeneralCommand = "setup";
export type BackendCommands = GeneralCommand | LibraryCommand | ReleaseCommand;
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

export interface BackendPathedError {
	path: string;
	error: BackendBaseError;
}

export interface BackendActionPayload<T, E> {
	type: "ok" | "error";
	data: T | E;
}

export interface BackendActionEntity<T> {
	id: number;
	attributes: T;
}
