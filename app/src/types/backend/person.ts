export type PersonType = "artist" | "composer" | "producer" | "unknown";

export interface Person {
	type: PersonType;
	name: string;
	name_sort: null | string;
	mbz_id: null | string;
}
