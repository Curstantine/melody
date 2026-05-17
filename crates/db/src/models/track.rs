use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::InlinePerson;

#[derive(Debug, Serialize, Deserialize)]
pub struct Track {
	pub title: String,
	pub title_sort: Option<String>,
	pub track_number: Option<u32>,
	pub disc_number: Option<u32>,
	pub original_date: Option<NaiveDate>,

	/// Either [InlinePerson::unknown] or populated with artists.
	pub artists: Vec<InlinePerson>,
	pub artist_sort: Option<String>,

	/// Either [Release::UNKNOWN_ID] or a u64 of sorts.
	pub release_id: u64,
	pub composer_ids: Option<Vec<u64>>,
	pub producer_ids: Option<Vec<u64>>,
	pub cover_ids: Option<Vec<u64>>,

	pub genre_ids: Option<Vec<u64>>,
	pub tag_ids: Option<Vec<u64>>,

	pub mbz_id: Option<String>,
	pub path: String,
}
