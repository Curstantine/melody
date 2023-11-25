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

impl TempTrack {
	pub fn into_track(
		self,
		artists: Option<Vec<InlinedArtist>>,
		release_id: Option<u64>,
		composer_ids: Option<Vec<u64>>,
		producer_ids: Option<Vec<u64>>,
		genre_ids: Option<Vec<u64>>,
		tag_ids: Option<Vec<u64>>,
	) -> Track {
		Track {
			title: self.title,
			title_sort: self.title_sort,
			track_number: self.track_number,
			disc_number: self.disc_number,
			original_date: self.original_date,
			artist_sort: self.artist_sort,
			mbz_id: self.mbz_id,
			path: self.path,

			artists,
			release_id,
			composer_ids,
			producer_ids,
			genre_ids,
			tag_ids,

			cover_ids: None,
		}
	}
}
