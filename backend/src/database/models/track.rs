use bonsaidb::core::schema::Collection;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::InlinedArtist;

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "tracks")]
pub struct Track {
	pub title: String,
	pub title_sort: Option<String>,
	pub track_number: Option<u32>,
	pub disc_number: Option<u32>,
	pub original_date: Option<NaiveDate>,

	pub artists: Option<Vec<InlinedArtist>>,
	pub artist_sort: Option<String>,

	pub release_id: Option<u64>,
	pub composer_ids: Option<Vec<u64>>,
	pub producer_ids: Option<Vec<u64>>,
	pub cover_ids: Option<Vec<u64>>,

	pub genre_ids: Option<Vec<u64>>,
	pub tag_ids: Option<Vec<u64>>,

	pub mbz_id: Option<String>,
	pub path: String,
}
