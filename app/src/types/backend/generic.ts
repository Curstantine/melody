export type CountryCode = "worldwide" | string;
export type ScriptCode = "latin" | "japanese";

export interface InlinedArtist {
	id: number;
	credited_as: null | string;
	join: string;
}
