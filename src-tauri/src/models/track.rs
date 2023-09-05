use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Track {
	pub title: String,
	pub title_sort: Option<String>,
	pub track_number: Option<u32>,

	/// Refers to the original release date of the track.
	///
	/// Usually, this is the same as the date property of [Release].
	pub original_date: Option<NaiveDate>,

	pub release_id: Option<String>,
	pub artist_ids: Option<Vec<String>>,
	pub composer_ids: Option<Vec<String>>,
	pub producer_ids: Option<Vec<String>>,

	/// Tag ids with [super::tag::TagType::Genre] as its type
	pub genre_ids: Option<Vec<String>>,
}
