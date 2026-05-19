use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Track {
	pub id: Uuid,
	pub title: String,
	pub title_sort: Option<String>,
	pub track_number: Option<u32>,
	pub disc_number: Option<u32>,
	pub original_date: Option<NaiveDate>,

	pub artist_sort: Option<String>,

	pub release_id: Option<Uuid>,
	pub cover_ids: Option<Vec<Uuid>>,
	pub mbz_id: Option<String>,

	pub path: String,
}

impl Track {
	pub fn as_new(&mut self) -> &Self {
		self.id = Uuid::now_v7();
		self
	}
}
