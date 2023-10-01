use std::fs::File;

use chrono::NaiveDate;
use symphonia::core::{
	formats::FormatOptions,
	io::MediaSourceStream,
	meta::{MetadataOptions, MetadataRevision, StandardTagKey, Value},
	probe::Hint,
};

use crate::{
	database::models::{
		person::{Person, PersonType},
		release::{ReleaseType, ReleaseTypeSecondary},
		tag::{Tag, TagType},
		CountryCode, FromTag, ScriptCode,
	},
	errors::{Error, ErrorType, Result},
	models::temp::TempTrackMeta,
};

pub fn read_track_meta(source: Box<File>, extension: Option<&str>) -> Result<TempTrackMeta> {
	let mss = MediaSourceStream::new(source, Default::default());
	let meta_opts: MetadataOptions = Default::default();
	let fmt_opts: FormatOptions = Default::default();
	let mut hint = Hint::new();

	if let Some(ext) = extension {
		hint.with_extension(ext);
	}

	let mut probed = symphonia::default::get_probe()
		.format(&hint, mss, &fmt_opts, &meta_opts)
		.expect("unsupported format");

	match probed.format.metadata().current() {
		Some(metadata) => traverse_meta(metadata),
		None => match probed.metadata.get().as_ref().and_then(|m| m.current()) {
			Some(metadata) => traverse_meta(metadata),
			None => Err(Error::descriptive("No metadata found for this file")),
		},
	}
}

fn traverse_meta(meta: &MetadataRevision) -> Result<TempTrackMeta> {
	let mut temp_meta = TempTrackMeta::default();

	let tags = meta.tags();
	if tags.is_empty() {
		return Err(Error::descriptive("Tags were empty for this file"));
	}

	let mut used_artists_field = false;
	let mut primary_release_type_used = false;

	println!("{:#?}", tags);
	for tag in tags {
		if let Some(key) = tag.std_key {
			match key {
				StandardTagKey::TrackTitle => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_track();
						x.title = val;
					}
				}
				StandardTagKey::SortTrackTitle => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_track();
						x.title_sort = Some(val);
					}
				}

				StandardTagKey::Artist if !used_artists_field => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.artists.get_or_insert_with(Vec::new);
						let y = Person {
							name: val,
							type_: PersonType::Artist,
							name_sort: None,
							mbz_id: None,
						};

						x.push(y);
					}
				}
				StandardTagKey::Composer => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.composers.get_or_insert_with(Vec::new);
						let y = Person {
							name: val,
							type_: PersonType::Composer,
							name_sort: None,
							mbz_id: None,
						};

						x.push(y);
					}
				}
				StandardTagKey::Producer => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.producers.get_or_insert_with(Vec::new);
						let y = Person {
							name: val,
							type_: PersonType::Producer,
							name_sort: None,
							mbz_id: None,
						};

						x.push(y);
					}
				}

				StandardTagKey::Album => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_release();
						x.name = val;
					}
				}
				StandardTagKey::SortAlbum => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_release();
						x.name_sort = Some(val);
					}
				}
				StandardTagKey::AlbumArtist => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_release();
						x.artists.get_or_insert_with(Vec::new).push(val);
					}
				}
				StandardTagKey::SortAlbumArtist => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_release();
						x.artist_sort = Some(val);
					}
				}

				StandardTagKey::Script => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_release();
						let y = ScriptCode::from_tag(val.as_str()).unwrap();
						x.script = Some(y);
					}
				}
				StandardTagKey::ReleaseCountry => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_release();
						let y = CountryCode::from_tag(val.as_str()).unwrap();
						x.country = Some(y);
					}
				}

				StandardTagKey::TrackNumber => {
					if let Some(val) = get_val_str_or_u32(&tag.value)? {
						let x = temp_meta.get_or_default_track();
						x.track_number = Some(val);
					}
				}
				StandardTagKey::DiscNumber => {
					if let Some(val) = get_val_str_or_u32(&tag.value)? {
						let x = temp_meta.get_or_default_track();
						x.disc_number = Some(val);
					}
				}
				StandardTagKey::TrackTotal => {
					if let Some(val) = get_val_str_or_u32(&tag.value)? {
						let x = temp_meta.get_or_default_release();
						x.total_tracks = Some(val);
					}
				}
				StandardTagKey::DiscTotal => {
					if let Some(val) = get_val_str_or_u32(&tag.value)? {
						let x = temp_meta.get_or_default_release();
						x.total_discs = Some(val);
					}
				}

				StandardTagKey::OriginalDate => {
					if let Some(val) = get_val_naive_date(&tag.value)? {
						let x = temp_meta.get_or_default_track();
						x.original_date = Some(val);
					}
				}
				StandardTagKey::Date if tag.key == "DATE" => {
					if let Some(val) = get_val_naive_date(&tag.value)? {
						let x = temp_meta.get_or_default_release();
						x.date = Some(val);
					}
				}
				StandardTagKey::Date if tag.key == "YEAR" => {
					if let Some(val) = get_val_str_or_u32(&tag.value)? {
						let x = temp_meta.get_or_default_release();
						x.year = Some(val);
					}
				}

				StandardTagKey::Label => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_release();
						x.labels.get_or_insert_with(Vec::new).push(val);
					}
				}
				StandardTagKey::IdentCatalogNumber => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_release();
						x.catalog_number = Some(val);
					}
				}

				StandardTagKey::Genre => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.genres.get_or_insert_with(Vec::new);
						let y = Tag {
							name: val,
							type_: TagType::Genre,
						};

						x.push(y);
					}
				}

				StandardTagKey::MusicBrainzRecordingId => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_track();
						x.mbz_id = Some(val);
					}
				}
				StandardTagKey::MusicBrainzAlbumId => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = temp_meta.get_or_default_release();
						x.mbz_id = Some(val);
					}
				}

				_ => {}
			}
		}

		#[allow(clippy::single_match)]
		match tag.key.as_str() {
			"ARTISTS" => {
				if let Some(val) = get_val_string(&tag.value) {
					let y = Person {
						name: val,
						type_: PersonType::Artist,
						name_sort: None,
						mbz_id: None,
					};

					// It's fine to overwrite the artists array, since the ARTISTS field *should* contain
					// all artists associated with the track.
					if !used_artists_field {
						used_artists_field = true;
						temp_meta.artists.replace(vec![y]);
					} else {
						let x = temp_meta.artists.get_or_insert_with(Vec::new);
						x.push(y);
					}
				}
			}

			// Symphonia for some reason doesn't support MusicBrainzReleaseType when a secondary tag is available,
			// and for a cursed reason, MusicBrainz adds the secondary type to the RELEASETYPE field along
			// with primary type.
			"RELEASETYPE" if !primary_release_type_used => {
				if let Some(val) = get_val_string(&tag.value) {
					let x = temp_meta.get_or_default_release();

					match ReleaseType::from_tag(val.as_str()) {
						Ok(y) => {
							x.type_ = y;
							primary_release_type_used = true;
						}
						Err(e) if e.type_ == ErrorType::Conversion => {
							let y = ReleaseTypeSecondary::from_tag(val.as_str())?;
							x.type_secondary.get_or_insert_with(Vec::new).push(y);
						}
						Err(e) => return Err(e),
					}
				}
			}
			"RELEASETYPE" if primary_release_type_used => {
				if let Some(val) = get_val_string(&tag.value) {
					let x = temp_meta.get_or_default_release();
					let y = ReleaseTypeSecondary::from_tag(val.as_str())?;
					x.type_secondary.get_or_insert_with(Vec::new).push(y);
				}
			}
			_ => {}
		}
	}

	Ok(temp_meta)
}

#[inline]
fn get_val_string(value: &Value) -> Option<String> {
	match value {
		Value::String(s) => Some(s.to_owned()),
		_ => None,
	}
}

#[inline]
fn get_val_str_or_u32(value: &Value) -> Result<Option<u32>> {
	match value {
		Value::String(s) => Ok(Some(s.parse::<u32>()?)),
		Value::UnsignedInt(x) => Ok(Some(*x as u32)),
		_ => Ok(None),
	}
}

#[inline]
fn get_val_naive_date(value: &Value) -> Result<Option<NaiveDate>> {
	match value {
		Value::String(s) => Ok(Some(NaiveDate::parse_from_str(s, "%Y-%m-%d")?)),
		_ => Ok(None),
	}
}

#[cfg(test)]
mod test {
	use std::fs::File;
	use std::path::Path;

	use crate::utils::symphonia::read_track_meta;

	const TRACK_PATH: &str =
		r"c:\Users\Curstantine\Music\TempLib\Kobaryo\SUPER DREAM ZONE\01 Kobaryo - Start of the Determination.flac";

	#[test]
	fn test_read_track_meta() {
		let path = Path::new(TRACK_PATH);
		let file = File::open(path).unwrap();
		let extension = path.extension().and_then(|s| s.to_str());

		let result = read_track_meta(Box::new(file), extension).unwrap();
		println!("{:#?}", result);
	}
}
