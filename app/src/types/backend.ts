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
