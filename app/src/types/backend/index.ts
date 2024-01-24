import type { LibraryCommand, LibraryEventType } from "@/types/backend/library";
import type { ReleaseCommand } from "@/types/backend/release";
import type { TrackCommand } from "@/types/backend/track";

export type GeneralCommand = "setup";
export type BackendCommands = GeneralCommand | LibraryCommand | ReleaseCommand | TrackCommand;
export type BackendEvents = LibraryEventType;

export interface BackendBaseError {
	short: string;
	message: null | string;
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
