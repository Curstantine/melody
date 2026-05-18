use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::{Uuid, uuid};

use super::{CountryCode, FromTag, ScriptCode, person::InlinePerson};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
	Album,
	Ep,
	Single,
	Broadcast,
	Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseTypeSecondary {
	Compilation,
	Remix,
	Live,
	Soundtrack,
	Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Release {
	pub id: Uuid,
	pub name: String,
	pub name_sort: Option<String>,

	pub year: Option<i32>,
	pub date: Option<NaiveDate>,
	pub country: Option<CountryCode>,
	pub script: Option<ScriptCode>,
	pub total_tracks: Option<u32>,
	pub catalog_number: Option<String>,

	pub artists: Vec<InlinePerson>,
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

impl Release {
	pub const UNKNOWN_ID: Uuid = uuid!("00000000-0000-0000-0000-ffff00000001");
}

impl FromTag for ReleaseType {
	type Error = std::convert::Infallible;

	fn from_tag(value: &str) -> Result<Self, Self::Error> {
		let value = match value.to_lowercase().as_str() {
			"album" => Self::Album,
			"ep" => Self::Ep,
			"single" => Self::Single,
			"broadcast" => Self::Broadcast,
			_ => Self::Other,
		};

		Ok(value)
	}
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
