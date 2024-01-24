import type { InlinedArtist } from "@/types/backend/generic";
import type { Person } from "@/types/backend/person";

export type TrackCommand = "get_track_list_for_release";

export interface Track {
	title: string;
	title_sort: string | null;
	track_number: number | null;
	disc_number: number | null;
	original_date: string | null;

	artists: InlinedArtist[];
	artist_sort: string | null;

	release_id: number;
	composer_ids: number[] | null;
	producer_ids: number[] | null;
	cover_ids: number[] | null;

	genre_ids: number[] | null;
	tag_ids: number[] | null;

	mbz_id: string | null;
	path: string;
}

export interface DisplayTrackList {
	tracks: Track[];
	artists: Record<number, Person>;
}

export interface GetTrackListParameters {
	[key: string]: unknown;
	releaseId: number;
}
