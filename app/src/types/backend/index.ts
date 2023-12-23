import type { LibraryCommand, LibraryEventType } from "@/types/backend/library";
import type { ReleaseCommand } from "@/types/backend/release";

export type GeneralCommand = "setup";
export type BackendCommands = GeneralCommand | LibraryCommand | ReleaseCommand;
export type BackendEvents = LibraryEventType;

export interface BackendBaseError {
	short: string;
	message?: string;
	data?: {
		type: "path" | "string";
		data: string;
	};
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
