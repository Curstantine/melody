use crate::{
	database::models::release::ReleaseType,
	models::temp::{
		cover::TempCover,
		label::TempLabel,
		person::{TempPerson, TempPersonCredit},
		release::TempRelease,
		tag::TempTag,
		track::TempTrack,
	},
};

pub mod cover;
pub mod label;
pub mod person;
pub mod release;
pub mod tag;
pub mod track;

/// Type representing a probable date in the (year, month, day) format.
pub type OptionedDate = Option<(Option<i32>, Option<u32>, Option<u32>)>;

#[derive(Debug, Default)]
pub struct TempTrackMeta {
	pub track: Option<TempTrack>,
	pub release: Option<TempRelease>,

	pub artists: Option<Vec<TempPersonCredit>>,
	pub release_artists: Option<Vec<TempPersonCredit>>,
	pub composers: Option<Vec<TempPerson>>,
	pub producers: Option<Vec<TempPerson>>,

	pub labels: Option<Vec<TempLabel>>,
	pub genres: Option<Vec<TempTag>>,
	pub tags: Option<Vec<TempTag>>,

	pub path: String,
}

impl TempTrackMeta {
	pub fn get_or_default_track(&mut self) -> &mut TempTrack {
		self.track.get_or_insert_with(|| TempTrack {
			title: String::with_capacity(0),
			title_sort: None,
			track_number: None,
			disc_number: None,
			original_date: None,
			artist_sort: None,
			mbz_id: None,
			path: String::with_capacity(0),
		})
	}

	pub fn get_or_default_release(&mut self) -> &mut TempRelease {
		self.release.get_or_insert_with(|| TempRelease {
			name: String::with_capacity(0),
			name_sort: None,
			year: None,
			date: None,
			country: None,
			script: None,
			total_tracks: None,
			total_discs: None,
			catalog_number: None,
			artist_sort: None,
			type_: ReleaseType::Album,
			type_secondary: None,
			mbz_id: None,
		})
	}
}

#[derive(Debug, Default)]
pub struct TempTrackResource {
	pub track_covers: Option<Vec<TempCover>>,
	pub release_covers: Option<Vec<TempCover>>,
}
