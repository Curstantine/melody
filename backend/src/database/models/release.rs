use bonsaidb::core::schema::Collection;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum CountryCode {
	XW,
	Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ScriptCode {
	Latn,
	Jpan,
	Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
	Album,
	Compilation,
	Ep,
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
#[serde(rename_all = "snake_case")]
pub enum ReleaseTypeSecondary {
	Compilation,
	Remix,
	Live,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseArtist {
	pub id: String,
	pub name: String,
	pub join: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "releases")]
pub struct Release {
	pub name: String,
	pub name_sort: Option<String>,

	pub year: Option<u32>,
	pub date: Option<NaiveDate>,
	pub country: Option<CountryCode>,
	pub total_tracks: Option<u32>,
	pub catalog_number: Option<String>,

	pub artist: Option<String>,
	pub artist_id: Option<String>,
	pub artist_sort: Option<String>,
	pub artists: Option<Vec<ReleaseArtist>>,

	pub label_ids: Option<Vec<String>>,
	pub genre_ids: Option<Vec<String>>,
	pub tag_ids: Option<Vec<String>>,

	#[serde(rename = "type")]
	pub type_: ReleaseType,
	pub type_secondary: Option<Vec<ReleaseTypeSecondary>>,

	pub mbz_id: Option<String>,
}
