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

export interface DisplayReleases {
	releases: Record<number, Release>;
	artists: Record<number, Person>;
	covers: Record<number, DisplayCover>;
}

export interface ReleasesGetParameters {
	[key: string]: unknown;
	libraryId: number;
}
