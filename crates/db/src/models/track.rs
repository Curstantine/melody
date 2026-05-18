use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::person::InlinePerson;

#[derive(Debug, Serialize, Deserialize)]
pub struct Track {
	pub id: Uuid,
	pub title: String,
	pub title_sort: Option<String>,
	pub track_number: Option<u32>,
	pub disc_number: Option<u32>,
	pub original_date: Option<NaiveDate>,

	pub artists: Vec<InlinePerson>,
	pub artist_sort: Option<String>,

	pub release_id: Uuid,
	pub composer_ids: Option<Vec<Uuid>>,
	pub producer_ids: Option<Vec<Uuid>>,
	pub cover_ids: Option<Vec<Uuid>>,

	pub genre_ids: Option<Vec<Uuid>>,
	pub tag_ids: Option<Vec<Uuid>>,

	pub mbz_id: Option<String>,
	pub path: String,
}

impl Track {
	pub fn temp(
		title: String,
		title_sort: Option<String>,
		track_number: Option<u32>,
		disc_number: Option<u32>,
		original_date: Option<NaiveDate>,
		artists: Vec<InlinePerson>,
		artist_sort: Option<String>,
		release_id: Uuid,
		composer_ids: Option<Vec<Uuid>>,
		producer_ids: Option<Vec<Uuid>>,
		cover_ids: Option<Vec<Uuid>>,
		genre_ids: Option<Vec<Uuid>>,
		tag_ids: Option<Vec<Uuid>>,
		mbz_id: Option<String>,
		path: String,
	) -> Self {
		Self {
			id: Uuid::nil(),
			title,
			title_sort,
			track_number,
			disc_number,
			original_date,
			artists,
			artist_sort,
			release_id,
			composer_ids,
			producer_ids,
			cover_ids,
			genre_ids,
			tag_ids,
			mbz_id,
			path,
		}
	}

	pub fn as_new(&mut self) -> &Self {
		self.id = Uuid::now_v7();
		self
	}
}
