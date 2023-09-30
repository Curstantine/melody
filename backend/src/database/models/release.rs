use bonsaidb::core::schema::Collection;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::{CountryCode, InlinedArtist, ScriptCode};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
	Album,
	Compilation,
	Ep,
	Single,
}

impl TryFrom<&str> for ReleaseType {
	type Error = crate::errors::Error;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		match value {
			"album" => Ok(Self::Album),
			"compilation" => Ok(Self::Compilation),
			"ep" => Ok(Self::Ep),
			"single" => Ok(Self::Single),
			_ => Err(Self::Error::conversion(format!("Invalid release type: {}", value))),
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

	pub artist: Option<String>,
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
