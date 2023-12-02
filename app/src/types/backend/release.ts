import type { BackendEntity } from "@/types/backend";
import type { CountryCode, InlinedArtist, ScriptCode } from "@/types/backend/generic";

export type ReleaseCommand = "get_releases";

export type ReleaseEntity = BackendEntity<Release>;

export type ReleaseType = "album" | "ep" | "single" | string;
export type ReleaseTypeSecondary = "compilation" | "remix" | "live" | "soundtrack" | string;

export interface Release {
	name: string;
	name_sort?: string;

	year?: number;
	date?: string;
	country?: CountryCode;
	script?: ScriptCode;
	total_tracks?: number;
	catalog_number?: string;

	artists: InlinedArtist[];
	artist_sort?: string;

	label_ids?: number[];
	genre_ids?: number[];
	tag_ids?: number[];
	cover_ids?: number[];

	type: ReleaseType;
	type_secondary?: ReleaseTypeSecondary;

	mbz_id?: string;
}

export interface ReleasesGetParameters {
	[key: string]: unknown;
	library_id: number;
}
