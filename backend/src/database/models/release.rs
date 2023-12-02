use std::borrow::Cow;

use bonsaidb::core::schema::Collection;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{database::views::release::ReleaseByNameAndArtist, errors::Error};

use super::{CountryCode, FromTag, InlinedArtist, ScriptCode};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
	Album,
	Ep,
	Single,
	Broadcast,
	Other,
}

impl FromTag for ReleaseType {
	type Error = Error;

	fn from_tag(value: &str) -> Result<Self, Self::Error> {
		let value = match value.to_lowercase().as_str() {
			"album" => Self::Album,
			"ep" => Self::Ep,
			"single" => Self::Single,
			"broadcast" => Self::Broadcast,
			"other" => Self::Other,
			_ => {
				let x = format!("Expected known release types, but got {}", value);
				return Err(Error::conversion("Failed to resolve release type", Some(Cow::Owned(x))));
			}
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
	Soundtrack,
	Other(String),
}

impl FromTag for ReleaseTypeSecondary {
	type Error = std::convert::Infallible;

	fn from_tag(value: &str) -> Result<Self, Self::Error> {
		let value = match value.to_lowercase().as_str() {
			"compilation" => Self::Compilation,
			"remix" => Self::Remix,
			"live" => Self::Live,
			"soundtrack" => Self::Soundtrack,
			x => Self::Other(x.to_string()),
		};

		Ok(value)
	}
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "releases", views = [ReleaseByNameAndArtist])]
pub struct Release {
	pub name: String,
	pub name_sort: Option<String>,

	pub year: Option<i32>,
	pub date: Option<NaiveDate>,
	pub country: Option<CountryCode>,
	pub script: Option<ScriptCode>,
	pub total_tracks: Option<u32>,
	pub catalog_number: Option<String>,

	pub artists: Vec<InlinedArtist>,
	pub artist_sort: Option<String>,

	pub label_ids: Option<Vec<u64>>,
	pub genre_ids: Option<Vec<u64>>,
	pub tag_ids: Option<Vec<u64>>,
	pub cover_ids: Option<Vec<u64>>,

	#[serde(rename = "type")]
	pub type_: ReleaseType,
	pub type_secondary: Option<Vec<ReleaseTypeSecondary>>,

	pub mbz_id: Option<String>,
}

#[cfg(test)]
impl Default for Release {
	fn default() -> Self {
		use crate::constants::TEST_RELEASE_NAME;

		Self {
			name: TEST_RELEASE_NAME.to_string(),
			name_sort: None,

			year: None,
			date: None,
			country: None,
			script: None,
			total_tracks: None,
			catalog_number: None,

			artists: vec![InlinedArtist::unknown()],
			artist_sort: None,

			label_ids: None,
			genre_ids: None,
			tag_ids: None,
			cover_ids: None,

			type_: ReleaseType::Album,
			type_secondary: None,

			mbz_id: None,
		}
	}
}
