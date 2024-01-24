import { InlinedArtist } from "@/types/backend/generic";

export interface Track {
	title: string;
	title_sort: string | null;
	track_number: number | null;
	disc_number: number | null;
	original_date: string | null;

	artists: InlinedArtist[] | null;
	artist_sort: string | null;

	release_id: number | null;
	composer_ids: number[] | null;
	producer_ids: number[] | null;
	cover_ids: number[] | null;

	genre_ids: number[] | null;
	tag_ids: number[] | null;

	mbz_id: string | null;
	path: string;
}
