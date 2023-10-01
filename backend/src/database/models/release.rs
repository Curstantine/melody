use bonsaidb::core::schema::Collection;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::{CountryCode, FromTag, InlinedArtist, ScriptCode};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
	Album,
	Compilation,
	Ep,
	Single,
}

impl FromTag for ReleaseType {
	type Error = crate::errors::Error;

	fn from_tag(value: &str) -> Result<Self, Self::Error> {
		let value = match value.to_lowercase().as_str() {
			"album" => Self::Album,
			"compilation" => Self::Compilation,
			"ep" => Self::Ep,
			"single" => Self::Single,
			_ => return Err(Self::Error::conversion(format!("Unknown release type: {}", value))),
		};

		Ok(value)
	}
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseTypeSecondary {
	Compilation,
	Remix,
	Live,
}

impl FromTag for ReleaseTypeSecondary {
	type Error = crate::errors::Error;

	fn from_tag(value: &str) -> Result<Self, Self::Error> {
		let value = match value.to_lowercase().as_str() {
			"compilation" => Self::Compilation,
			"remix" => Self::Remix,
			"live" => Self::Live,
			_ => {
				return Err(Self::Error::conversion(format!(
					"Unknown release type secondary: {}",
					value
				)))
			}
		};

		Ok(value)
	}
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "releases")]
pub struct Release {
	pub name: String,
	pub name_sort: Option<String>,

	pub year: Option<u32>,
	pub date: Option<NaiveDate>,
	pub country: Option<CountryCode>,
	pub script: Option<ScriptCode>,
	pub total_tracks: Option<u32>,
	pub catalog_number: Option<String>,

	pub artist_id: Option<String>,
	pub artist_sort: Option<String>,
	pub artists: Option<Vec<InlinedArtist>>,

	pub label_ids: Option<Vec<String>>,
	pub genre_ids: Option<Vec<String>>,
	pub tag_ids: Option<Vec<String>>,

	#[serde(rename = "type")]
	pub type_: ReleaseType,
	pub type_secondary: Option<Vec<ReleaseTypeSecondary>>,

	pub mbz_id: Option<String>,
}
