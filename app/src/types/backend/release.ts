import type { BackendEntity } from "@/types/backend";
import type { DisplayCover } from "@/types/backend/cover";
import type { CountryCode, InlinedArtist, ScriptCode } from "@/types/backend/generic";
import type { Person } from "@/types/backend/person";

export type ReleaseCommand = "get_releases" | "get_display_releases";

export type ReleaseEntity = BackendEntity<Release>;

export type ReleaseType = "album" | "ep" | "single" | string;
export type ReleaseTypeSecondary = "compilation" | "remix" | "live" | "soundtrack" | string;

export interface Release {
	name: string;
	name_sort: null | string;

	year: null | number;
	date: null | string;
	country: null | CountryCode;
	script: null | ScriptCode;
	total_tracks: null | number;
	catalog_number: null | string;

	artists: InlinedArtist[];
	artist_sort: null | string;

	label_ids: null | number[];
	genre_ids: null | number[];
	tag_ids: null | number[];
	cover_ids: null | number[];

	type: ReleaseType;
	type_secondary: null | ReleaseTypeSecondary;

	mbz_id: null | string;
}

export interface DisplayReleases {
	releases: Record<number, Release>;
	artists: Record<number, Person>;
	covers: Record<number, DisplayCover>;
}

export interface ReleasesGetParameters {
	[key: string]: unknown;
	libraryId: number;
}
