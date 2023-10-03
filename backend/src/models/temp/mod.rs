use crate::database::models::{label::Label, person::Person, release::ReleaseType, tag::Tag, InlinedArtist};

use self::{release::TempRelease, track::TempTrack};

pub mod release;
pub mod track;

#[derive(Debug)]
pub struct TempInlinedArtist {
	pub person: Person,
	pub credited_as: Option<String>,
	pub join: Option<String>,
}

impl From<Person> for TempInlinedArtist {
	fn from(person: Person) -> Self {
		Self {
			person,
			credited_as: None,
			join: None,
		}
	}
}

impl TempInlinedArtist {
	pub fn into_inlined(self, id: u64) -> InlinedArtist {
		InlinedArtist {
			id,
			credited_as: self.credited_as,
			join: self.join,
		}
	}
}

#[derive(Debug, Default)]
pub struct TempTrackMeta {
	pub track: Option<TempTrack>,
	pub release: Option<TempRelease>,

	pub artists: Option<Vec<TempInlinedArtist>>,
	pub release_artists: Option<Vec<TempInlinedArtist>>,
	pub composers: Option<Vec<Person>>,
	pub producers: Option<Vec<Person>>,

	pub labels: Option<Vec<Label>>,
	pub genres: Option<Vec<Tag>>,
	pub tags: Option<Vec<Tag>>,
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
