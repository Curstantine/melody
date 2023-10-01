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

	pub artist_id: Option<String>,
	pub artist_sort: Option<String>,
	pub artists: Option<Vec<InlinedArtist>>,

	pub release_id: Option<String>,
	pub composer_ids: Option<Vec<String>>,
	pub producer_ids: Option<Vec<String>>,

	pub genre_ids: Option<Vec<String>>,
	pub tag_ids: Option<Vec<String>>,

	pub mbz_id: Option<String>,
}
