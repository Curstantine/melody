use chrono::NaiveDate;

#[derive(Debug)]
pub struct TempTrack {
	pub title: String,
	pub title_sort: Option<String>,
	pub track_number: Option<u32>,
	pub disc_number: Option<u32>,
	pub original_date: Option<NaiveDate>,
	pub mbz_id: Option<String>,
}
