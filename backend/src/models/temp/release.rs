use chrono::NaiveDate;

use crate::database::models::{
	release::{ReleaseType, ReleaseTypeSecondary},
	CountryCode, ScriptCode,
};

#[derive(Debug)]
pub struct TempRelease {
	pub name: String,
	pub name_sort: Option<String>,

	pub year: Option<u32>,
	pub date: Option<NaiveDate>,
	pub country: Option<CountryCode>,
	pub script: Option<ScriptCode>,
	pub total_tracks: Option<u32>,
	pub total_discs: Option<u32>,
	pub catalog_number: Option<String>,

	pub artists: Option<Vec<String>>,
	pub artist_sort: Option<String>,

	pub labels: Option<Vec<String>>,
	pub genres: Option<Vec<String>>,
	pub tags: Option<Vec<String>>,

	pub type_: ReleaseType,
	pub type_secondary: Option<Vec<ReleaseTypeSecondary>>,

	pub mbz_id: Option<String>,
}
