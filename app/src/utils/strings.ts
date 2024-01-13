import type { LibraryEvent, LibraryEventData } from "@/types/backend/library";

export function getLibraryEventTypeString(type: LibraryEvent["type"]): string {
	switch (type) {
		case "indexing":
			return "Indexing";
		case "reading":
			return "Reading";
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
