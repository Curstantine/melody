use {
	bonsaidb::core::schema::Collection,
	chrono::NaiveDate,
	serde::{Deserialize, Serialize},
};

use crate::database::{models::InlinedArtist, views::track::TrackByReleaseId};

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "tracks", views = [TrackByReleaseId])]
pub struct Track {
	pub title: String,
	pub title_sort: Option<String>,
	pub track_number: Option<u32>,
	pub disc_number: Option<u32>,
	pub original_date: Option<NaiveDate>,

	/// Either [InlinedArtist::unknown] or populated with artists.
	pub artists: Vec<InlinedArtist>,
	pub artist_sort: Option<String>,

	/// Either [constants::UNKNOWN_RELEASE_ID] or a u64 of sorts.
	pub release_id: u64,
	pub composer_ids: Option<Vec<u64>>,
	pub producer_ids: Option<Vec<u64>>,
	pub cover_ids: Option<Vec<u64>>,

	pub genre_ids: Option<Vec<u64>>,
	pub tag_ids: Option<Vec<u64>>,

	pub mbz_id: Option<String>,
	pub path: String,
}
