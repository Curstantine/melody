use uuid::Uuid;

use crate::models::{
	label::Label,
	release::{Release, ReleaseType},
	tag::Tag,
	temp::{cover::TempCover, person::TempInlinePerson},
	track::Track,
};

pub mod cover;
pub mod person;

/// Type representing a probable date in the (year, month, day) format.
pub type OptionedDate = Option<(Option<i32>, Option<u32>, Option<u32>)>;

#[derive(Debug, Default)]
pub struct TempTrackMeta {
	pub track: Option<Track>,
	pub release: Option<Release>,

	pub artists: Option<Vec<TempInlinePerson>>,
	pub release_artists: Option<Vec<TempInlinePerson>>,
	pub weak_artists: Option<Vec<TempInlinePerson>>,

	pub labels: Option<Vec<Label>>,
	pub tags: Option<Vec<Tag>>,
}

impl TempTrackMeta {
	pub fn get_or_default_track(&mut self) -> &mut Track {
		self.track.get_or_insert_with(|| Track {
			id: Uuid::nil(),
			title: String::with_capacity(0),
			title_sort: None,
			track_number: None,
			disc_number: None,
			original_date: None,
			artist_sort: None,
			release_id: None,
			mbz_id: None,
			cover_ids: None,
			path: String::with_capacity(0),
		})
	}

	pub fn get_or_default_release(&mut self) -> &mut Release {
		self.release.get_or_insert_with(|| Release {
			id: Release::UNKNOWN_ID,
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
			cover_ids: None,
		})
	}
}

#[derive(Debug, Default)]
pub struct TempTrackResource {
	pub track_covers: Option<Vec<TempCover>>,
	pub release_covers: Option<Vec<TempCover>>,
}
