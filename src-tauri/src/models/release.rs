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
	#[serde(rename = "album")]
	Album,
	#[serde(rename = "compilation")]
	Compilation,
	#[serde(rename = "ep")]
	Ep,
	#[serde(rename = "single")]
	Single,
}

impl ReleaseType {
	pub fn from_str(s: &str) -> Option<Self> {
		match s.to_lowercase().as_str() {
			"album" => Some(Self::Album),
			"compilation" => Some(Self::Compilation),
			"ep" => Some(Self::Ep),
			"single" => Some(Self::Single),
			_ => None,
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Release {
	pub name: String,
	pub name_sort: Option<String>,

	pub year: Option<u32>,
	pub date: Option<NaiveDate>,
	pub country: Option<CountryCode>,
	pub total_tracks: Option<u32>,
	pub catalog_number: Option<String>,

	/// Corresponds to the artist tag.
	/// This field can contain multiple artists, separated by commas, feat. prefixes and the like.
	///
	/// Use this field to display the artist name in the UI.
	pub display_artist: Option<String>,

	pub label_ids: Option<Vec<String>>,
	pub artist_ids: Option<Vec<String>>,
	pub tag_ids: Option<Vec<String>>,

	#[serde(rename = "type")]
	pub type_: ReleaseType,
}
