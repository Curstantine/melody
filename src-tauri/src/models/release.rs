use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum CountryCode {
	XW,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ScriptCode {
	Latn,
	Jpan,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ReleaseType {
	Album,
	Compilation,
	Ep,
	Single,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Release {
	pub name: String,
	pub name_sort: Option<String>,

	pub year: Option<u32>,
	pub date: Option<NaiveDate>,
	pub country: Option<CountryCode>,
	pub total_tracks: Option<u32>,

	pub label_ids: Option<Vec<String>>,
	pub artist_ids: Option<Vec<String>>,
	pub tag_ids: Option<Vec<String>>,

	#[serde(rename = "type")]
	pub type_: ReleaseType,
}
