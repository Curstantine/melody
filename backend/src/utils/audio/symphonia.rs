use std::{borrow::Cow, fs::File, path::Path};

use chrono::NaiveDate;
use symphonia::core::{
	formats::FormatOptions,
	io::MediaSourceStream,
	meta::{MetadataOptions, StandardTagKey, StandardVisualKey, Tag as SymphoniaTag, Value, Visual as SymphoniaVisual},
	probe::Hint,
};
use tracing::debug;

use crate::{
	database::models::{
		label::Label,
		person::{Person, PersonType},
		release::{ReleaseType, ReleaseTypeSecondary},
		resource::{ResourceMediaType, ResourceRelationType, ResourceType},
		tag::{Tag, TagType},
		CountryCode, FromTag, ScriptCode,
	},
	errors::{self, Error, ErrorKind, Result},
	models::temp::{resource::TempResource, OptionedDate, TempInlinedArtist, TempTrackMeta, TempTrackResource},
	utils::matchers,
};

pub fn read_track_meta(path: &Path) -> Result<(TempTrackMeta, TempTrackResource)> {
	let extension = path.extension().and_then(|s| s.to_str());
	let source = File::open(path)?;

	let path_str = path.to_str().unwrap();

	let mss = MediaSourceStream::new(Box::new(source), Default::default());
	let meta_opts: MetadataOptions = Default::default();
	let fmt_opts: FormatOptions = Default::default();
	let mut hint = Hint::new();

	if let Some(ext) = extension {
		hint.with_extension(ext);
	}

	let mut probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;
	let mut format = probed.format;

	let mut meta = TempTrackMeta {
		path: path_str.to_string(),
		..Default::default()
	};
	let mut resources = TempTrackResource::default();

	if let Some(rev) = format.metadata().current() {
		traverse_tags(&mut meta, rev.tags())?;
		traverse_visuals(&mut resources, rev.visuals())?;

		#[cfg(debug_assertions)]
		if probed.metadata.get().as_ref().is_some() {
			debug!("Tags found while probing that are not part of the container are ignored.")
		}
	} else if let Some(rev) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
		traverse_tags(&mut meta, rev.tags())?;
		traverse_visuals(&mut resources, rev.visuals())?;
	} else {
		return Err(errors::pre::probe_no_meta());
	}

	Ok((meta, resources))
}

fn traverse_visuals(resource: &mut TempTrackResource, visuals: &[SymphoniaVisual]) -> Result<()> {
	for visual in visuals {
		if let Some(StandardVisualKey::FrontCover) = visual.usage {
			let x = resource.release_covers.get_or_insert_with(Vec::new);
			let y = TempResource {
				type_: ResourceType::Image,
				relation_type: ResourceRelationType::Release,
				media_type: ResourceMediaType::from_tag(&visual.media_type)?,
				data: visual.data.clone(),
			};

			x.push(y);
		}
	}

	Ok(())
}

fn traverse_tags(meta: &mut TempTrackMeta, tags: &[SymphoniaTag]) -> Result<()> {
	let mut used_artists_field = false;
	let mut primary_release_type_used = false;

	if tags.is_empty() {
		return Err(errors::pre::probe_no_tags());
	}

	for tag in tags {
		let mut use_str_tag = false;

		match tag.std_key {
			Some(key) => match key {
				StandardTagKey::TrackTitle => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_track();
						x.title = val;
					}
				}
				StandardTagKey::SortTrackTitle => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_track();
						x.title_sort = Some(val);
					}
				}

				StandardTagKey::Artist if !used_artists_field => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.artists.get_or_insert_with(Vec::new);
						let y = Person {
							name: val,
							type_: PersonType::Artist,
							name_sort: None,
							mbz_id: None,
						};

						x.push(TempInlinedArtist::from(y));
					}
				}
				StandardTagKey::SortArtist => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_track();
						x.artist_sort = Some(val);
					}
				}
				StandardTagKey::Composer => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.composers.get_or_insert_with(Vec::new);
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
						let x = meta.producers.get_or_insert_with(Vec::new);
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
						let x = meta.get_or_default_release();
						x.name = val;
					}
				}
				StandardTagKey::SortAlbum => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_release();
						x.name_sort = Some(val);
					}
				}
				StandardTagKey::AlbumArtist => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.release_artists.get_or_insert_with(Vec::new);
						let y = Person {
							name: val,
							type_: PersonType::Artist,
							name_sort: None,
							mbz_id: None,
						};

						x.push(TempInlinedArtist::from(y));
					}
				}
				StandardTagKey::SortAlbumArtist => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_release();
						x.artist_sort = Some(val);
					}
				}

				StandardTagKey::Script => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_release();
						let y = ScriptCode::from_tag(val.as_str()).unwrap();
						x.script = Some(y);
					}
				}
				StandardTagKey::ReleaseCountry => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_release();
						let y = CountryCode::from_tag(val.as_str()).unwrap();
						x.country = Some(y);
					}
				}

				StandardTagKey::TrackNumber => {
					if let Some((track_no, track_total_opt)) = get_no_and_maybe_total(&tag.value)? {
						let y = meta.get_or_default_track();
						y.track_number = Some(track_no);

						if let Some(track_total) = track_total_opt {
							let z = meta.get_or_default_release();
							z.total_tracks.get_or_insert(track_total);
						}
					}
				}
				StandardTagKey::DiscNumber => {
					if let Some((disc_no, disc_total_opt)) = get_no_and_maybe_total(&tag.value)? {
						let y = meta.get_or_default_track();
						y.disc_number = Some(disc_no);

						if let Some(disc_total) = disc_total_opt {
							let z = meta.get_or_default_release();
							z.total_discs.get_or_insert(disc_total);
						}
					}
				}

				StandardTagKey::TrackTotal => {
					if let Some(val) = get_val_u32(&tag.value)? {
						let x = meta.get_or_default_release();
						x.total_tracks = Some(val);
					}
				}
				StandardTagKey::DiscTotal => {
					if let Some(val) = get_val_u32(&tag.value)? {
						let x = meta.get_or_default_release();
						x.total_discs = Some(val);
					}
				}

				StandardTagKey::OriginalDate => {
					if let Some((Some(year), Some(month), day_opt)) = get_val_date(&tag.value)? {
						let y = meta.get_or_default_track();
						y.original_date = NaiveDate::from_ymd_opt(year, month, day_opt.unwrap_or(1));
					}
				}
				StandardTagKey::Date => match get_val_date(&tag.value)? {
					Some((Some(year), Some(month), day_opt)) => {
						let y = meta.get_or_default_release();
						y.date = NaiveDate::from_ymd_opt(year, month, day_opt.unwrap_or(1));
					}
					Some((Some(year), None, None)) => {
						let y = meta.get_or_default_release();
						if y.year.is_none() {
							y.year = Some(year);
						}
					}
					_ => {}
				},

				StandardTagKey::Label => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.labels.get_or_insert_with(Vec::new);
						let y = Label { name: val };
						x.push(y);
					}
				}
				StandardTagKey::IdentCatalogNumber => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_release();
						x.catalog_number = Some(val);
					}
				}

				StandardTagKey::Genre => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.genres.get_or_insert_with(Vec::new);
						let y = Tag {
							name: val,
							type_: TagType::Genre,
						};

						x.push(y);
					}
				}

				StandardTagKey::MusicBrainzRecordingId => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_track();
						x.mbz_id = Some(val);
					}
				}
				StandardTagKey::MusicBrainzAlbumId => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_release();
						x.mbz_id = Some(val);
					}
				}

				_ => use_str_tag = true,
			},
			None => use_str_tag = true,
		}

		if use_str_tag {
			match tag.key.as_str() {
				"ARTISTS" => {
					if let Some(val) = get_val_string(&tag.value) {
						let y = TempInlinedArtist::from(Person {
							name: val,
							type_: PersonType::Artist,
							name_sort: None,
							mbz_id: None,
						});

						// It's fine to overwrite the artists array, since the ARTISTS field *should* contain
						// all artists associated with the track.
						if !used_artists_field {
							used_artists_field = true;
							meta.artists.replace(vec![y]);
						} else {
							let x = meta.artists.get_or_insert_with(Vec::new);
							x.push(y);
						}
					}
				}

				// Symphonia for some reason doesn't support MusicBrainzReleaseType when a secondary tag is available,
				// and for a cursed reason, MusicBrainz adds the secondary type to the RELEASETYPE field along
				// with primary type.
				"RELEASETYPE" if !primary_release_type_used => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_release();

						match ReleaseType::from_tag(val.as_str()) {
							Ok(y) => {
								x.type_ = y;
								primary_release_type_used = true;
							}
							Err(_) => {
								let y = ReleaseTypeSecondary::from_tag(val.as_str()).unwrap(); // Infallible
								x.type_secondary.get_or_insert_with(Vec::new).push(y);
							}
						}
					}
				}
				"RELEASETYPE" if primary_release_type_used => {
					if let Some(val) = get_val_string(&tag.value) {
						let x = meta.get_or_default_release();
						let y = ReleaseTypeSecondary::from_tag(val.as_str()).unwrap();
						x.type_secondary.get_or_insert_with(Vec::new).push(y);
					}
				}
				_ => continue,
			}
		}
	}

	Ok(())
}

#[inline]
fn get_val_string(value: &Value) -> Option<String> {
	match value {
		Value::String(s) => Some(s.to_owned()),
		_ => None,
	}
}

#[inline]
fn get_val_u32(value: &Value) -> Result<Option<u32>> {
	match value {
		Value::String(x) => {
			let y = crate::parse_str!(x, u32)?;
			Ok(Some(y))
		}
		Value::UnsignedInt(x) => Ok(Some(*x as u32)),
		_ => Ok(None),
	}
}

#[inline]
fn get_val_date(value: &Value) -> Result<OptionedDate> {
	let date: OptionedDate = match value {
		Value::String(x) if matchers::reg::is_ymd(x.as_str()) => {
			let splits = x.split('-').collect::<Vec<&str>>();

			let year = {
				let y = splits.first().unwrap();
				crate::parse_str!(y, i32)?
			};
			let month = {
				let y = splits.get(1).unwrap();
				crate::parse_str!(y, u32)?
			};
			let day = {
				let y = splits.get(2).unwrap();
				crate::parse_str!(y, u32)?
			};

			Some((Some(year), Some(month), Some(day)))
		}
		Value::String(x) if matchers::reg::is_ym(x.as_str()) => {
			let splits = x.split('-').collect::<Vec<&str>>();

			let year = {
				let y = splits.first().unwrap();
				crate::parse_str!(y, i32)?
			};
			let month = {
				let y = splits.get(1).unwrap();
				crate::parse_str!(y, u32)?
			};

			Some((Some(year), Some(month), None))
		}
		Value::String(x) if matchers::reg::is_year(x.as_str()) => {
			let year = crate::parse_str!(x, i32)?;
			Some((Some(year), None, None))
		}
		Value::UnsignedInt(x) => Some((Some(*x as i32), None, None)),
		_ => None,
	};

	Ok(date)
}

/// Reads into a value and tries to get an int followed by an optional int separated by a forward slash.
///
/// Useful for handling edge cases like track_no and track_total included in the same tag.
///
/// ### Example
/// ```
/// "2" -> (2, None)
/// "1/2" -> (1, None)
/// ```
#[inline]
fn get_no_and_maybe_total(value: &Value) -> Result<Option<(u32, Option<u32>)>> {
	let tuple: Option<(u32, Option<u32>)> = match value {
		Value::String(x) if matchers::reg::is_no_and_total(x) => {
			let splits = x.split('/').collect::<Vec<&str>>();
			let no_str = splits.first().unwrap();
			let total_str = splits.last().unwrap();

			let no = crate::parse_str!(no_str, u32)?;
			let total = crate::parse_str!(total_str, u32)?;

			Some((no, Some(total)))
		}
		Value::String(x) => {
			let y = crate::parse_str!(x, u32)?;
			Some((y, None))
		}
		Value::UnsignedInt(x) => Some((*x as u32, None)),
		_ => None,
	};

	Ok(tuple)
}

impl From<symphonia::core::errors::Error> for Error {
	fn from(value: symphonia::core::errors::Error) -> Self {
		use symphonia::core::errors::Error as SE;

		let (short, message): (&'static str, Cow<'static, str>) = match value {
			SE::DecodeError(x) => (
				"Symphonia: Decode failure",
				Cow::Owned(format!("The stream is either malformed or could not be decoded. {x}")),
			),
			SE::Unsupported(x) => {
				let y = format!("Symphonia was invoked with an unsupported codec/container feature: {x}");
				("Symphonia: Unsupported feature", Cow::Owned(y))
			}
			SE::IoError(x) => {
				let e = Error::from(x);
				("Symphonia: IO error", Cow::Owned(e.to_string()))
			}
			_ => ("Symphonia: Unhandled error", Cow::Owned(value.to_string())),
		};

		Self {
			kind: ErrorKind::Encoder,
			short: Cow::Borrowed(short),
			message: Some(message),
		}
	}
}

#[cfg(test)]
mod test {
	use std::path::Path;

	use super::read_track_meta;
	use crate::errors::Result;

	const TRACK_PATH: &str = r"C:\Users\Curstantine\Music\TempLib\Annabel\caracol\10 glimmer.flac";

	#[test]
	fn test_read_track_meta() -> Result<()> {
		let path = Path::new(TRACK_PATH);
		let result = read_track_meta(path)?;
		println!("{:#?}", result);

		Ok(())
	}
}
