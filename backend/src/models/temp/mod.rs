use crate::database::models::{person::Person, release::ReleaseType, tag::Tag};

use self::{release::TempRelease, track::TempTrack};

pub mod release;
pub mod track;

#[derive(Debug, Default)]
pub struct TempTrackMeta {
	pub track: Option<TempTrack>,
	pub release: Option<TempRelease>,

	pub artists: Option<Vec<Person>>,
	pub composers: Option<Vec<Person>>,
	pub producers: Option<Vec<Person>>,

	pub genres: Option<Vec<Tag>>,
	pub tags: Option<Vec<Tag>>,
}

impl TempTrackMeta {
	pub fn default_track(&mut self) -> &mut TempTrack {
		self.track.get_or_insert_with(|| TempTrack {
			title: String::with_capacity(0),
			title_sort: None,
			track_number: None,
			disc_number: None,
			original_date: None,
			mbz_id: None,
		})
	}

	pub fn default_release(&mut self) -> &mut TempRelease {
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
			artists: None,
			artist_sort: None,
			labels: None,
			genres: None,
			tags: None,
			type_: ReleaseType::Album,
			type_secondary: None,
			mbz_id: None,
		})
	}
}
