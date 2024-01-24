import type { InlinedArtist } from "@/types/backend/generic";
import type { LibraryEvent, LibraryEventData } from "@/types/backend/library";
import type { Person } from "@/types/backend/person";

export function getLibraryEventTypeString(type: LibraryEvent["type"]): string {
	switch (type) {
		case "indexing":
			return "Indexing";
		case "scanning":
			return "Scanning";
	}
}
export function getLibraryEventDataString(data: LibraryEventData): string {
	const { current, total } = data;
	return `(${current}/${total}):`;
}

export function getLibraryEventPath(event: LibraryEvent) {
	if (event.type === "scanning") {
		return event.data as string;
	}

	const { path } = event.data as LibraryEventData;
	return path;
}

// TODO: Support linking using id
export function joinInlinedArtists(inlined: InlinedArtist[], artists: Record<number, Person>): string {
	return inlined.map(({ id, join, credited_as }) => `${credited_as ?? artists[id].name}${join ?? ""}`).join("");
}
