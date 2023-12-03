export type PersonType = "artist" | "composer" | "producer" | "unknown";

export interface Person {
	type: PersonType;
	name: string;
	name_sort?: string;
	mbz_id?: string;
}
