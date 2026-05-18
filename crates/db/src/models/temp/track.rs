use chrono::NaiveDate;
use uuid::Uuid;

use crate::models::{person::InlinePerson, release::Release, track::Track};

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
	pub artists: Option<Vec<InlinePerson>>,
	pub release_id: Option<Uuid>,
	pub composer_ids: Option<Vec<Uuid>>,
	pub producer_ids: Option<Vec<Uuid>>,
	pub genre_ids: Option<Vec<Uuid>>,
	pub tag_ids: Option<Vec<Uuid>>,
	pub cover_ids: Option<Vec<Uuid>>,
}

impl TempTrack {
	pub fn into_track(self, arg: TempTrackIntoArg) -> Track {
		Track {
			id: Uuid::nil(),
			title: self.title,
			title_sort: self.title_sort,
			track_number: self.track_number,
			disc_number: self.disc_number,
			original_date: self.original_date,
			artist_sort: self.artist_sort,
			mbz_id: self.mbz_id,
			path: self.path,

			artists: arg.artists.unwrap_or_else(|| vec![InlinePerson::unknown()]),
			release_id: arg.release_id.unwrap_or(Release::UNKNOWN_ID),
			composer_ids: arg.composer_ids,
			producer_ids: arg.producer_ids,
			genre_ids: arg.genre_ids,
			tag_ids: arg.tag_ids,
			cover_ids: arg.cover_ids,
		}
	}
}
