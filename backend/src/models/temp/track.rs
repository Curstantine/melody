use chrono::NaiveDate;

use crate::database::models::{track::Track, InlinedArtist};

#[derive(Debug)]
pub struct TempTrack {
	pub title: String,
	pub title_sort: Option<String>,
	pub track_number: Option<u32>,
	pub disc_number: Option<u32>,
	pub original_date: Option<NaiveDate>,
	pub artist_sort: Option<String>,
	pub mbz_id: Option<String>,
	pub path: String,
}

pub struct TempTrackIntoArg {
	pub artists: Vec<InlinedArtist>,
	pub release_id: Option<u64>,
	pub composer_ids: Option<Vec<u64>>,
	pub producer_ids: Option<Vec<u64>>,
	pub genre_ids: Option<Vec<u64>>,
	pub tag_ids: Option<Vec<u64>>,
	pub cover_ids: Option<Vec<u64>>,
	pub library_ids: Vec<u32>,
}

impl TempTrack {
	pub fn into_track(self, arg: TempTrackIntoArg) -> Track {
		Track {
			title: self.title,
			title_sort: self.title_sort,
			track_number: self.track_number,
			disc_number: self.disc_number,
			original_date: self.original_date,
			artist_sort: self.artist_sort,
			mbz_id: self.mbz_id,
			path: self.path,

			artists: arg.artists,
			release_id: arg.release_id,
			composer_ids: arg.composer_ids,
			producer_ids: arg.producer_ids,
			genre_ids: arg.genre_ids,
			tag_ids: arg.tag_ids,
			cover_ids: arg.cover_ids,
			library_ids: arg.library_ids,
		}
	}
}
