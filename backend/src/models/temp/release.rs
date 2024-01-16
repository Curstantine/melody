use chrono::NaiveDate;

use crate::database::models::{
	release::{Release, ReleaseType, ReleaseTypeSecondary},
	CountryCode, InlinedArtist, ScriptCode,
};

#[derive(Debug)]
pub struct TempRelease {
	pub name: String,
	pub name_sort: Option<String>,

	pub year: Option<i32>,
	pub date: Option<NaiveDate>,
	pub country: Option<CountryCode>,
	pub script: Option<ScriptCode>,
	pub total_tracks: Option<u32>,
	pub total_discs: Option<u32>,
	pub catalog_number: Option<String>,
	pub artist_sort: Option<String>,

	pub type_: ReleaseType,
	pub type_secondary: Option<Vec<ReleaseTypeSecondary>>,

	pub mbz_id: Option<String>,
}

pub struct TempReleaseIntoArg {
	pub library_ids: Vec<u32>,
	pub artists: Option<Vec<InlinedArtist>>,
	pub label_ids: Option<Vec<u64>>,
	pub genre_ids: Option<Vec<u64>>,
	pub tag_ids: Option<Vec<u64>>,
	pub cover_ids: Option<Vec<u64>>,
}

impl TempRelease {
	pub fn into_release(self, arg: TempReleaseIntoArg) -> Release {
		Release {
			name: self.name,
			name_sort: self.name_sort,
			year: self.year,
			date: self.date,
			country: self.country,
			script: self.script,
			total_tracks: self.total_tracks,
			catalog_number: self.catalog_number,
			type_: self.type_,
			type_secondary: self.type_secondary,
			artist_sort: self.artist_sort,
			mbz_id: self.mbz_id,

			artists: arg.artists.unwrap_or_else(|| vec![InlinedArtist::unknown()]),
			label_ids: arg.label_ids,
			genre_ids: arg.genre_ids,
			tag_ids: arg.tag_ids,
			cover_ids: arg.cover_ids,
			library_ids: arg.library_ids,
		}
	}
}
