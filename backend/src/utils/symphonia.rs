use std::fs::File;

use chrono::NaiveDate;
use symphonia::core::{
	formats::FormatOptions,
	io::MediaSourceStream,
	meta::{MetadataOptions, MetadataRevision, StandardTagKey, Value},
	probe::Hint,
};

use crate::{
	database::models::release::ReleaseType,
	errors::{Error, Result},
};

pub fn read_track_meta(source: Box<File>, extension: Option<&str>) -> Result<TempMeta> {
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

#[derive(Debug, Default)]
pub struct TempMeta {
	pub title: Option<String>,
	pub title_sort: Option<String>,

	pub artist: Option<String>,
	pub artist_sort: Option<String>,

	pub artists: Vec<String>,
	pub composers: Vec<String>,
	pub producers: Vec<String>,
	pub label: Option<String>,

	pub release: Option<String>,
	pub release_type: Option<ReleaseType>,

	pub catalog_number: Option<String>,

	pub album_artists: Vec<String>,
	pub album_artist_sort: Vec<String>,

	pub year: Option<u32>,
	pub date: Option<NaiveDate>,

	pub original_year: Option<u32>,
	pub original_date: Option<NaiveDate>,

	pub disc_number: Option<u32>,
	pub total_discs: Option<u32>,

	pub track_number: Option<u32>,
	pub total_tracks: Option<u32>,

	pub genres: Vec<String>,
}

fn get_val_string(value: &Value) -> Option<String> {
	match value {
		Value::String(s) => Some(s.to_owned()),
		_ => None,
	}
}

fn get_val_str_or_u32(value: &Value) -> Result<Option<u32>> {
	match value {
		Value::String(s) => Ok(Some(s.parse::<u32>()?)),
		Value::UnsignedInt(x) => Ok(Some(*x as u32)),
		_ => Ok(None),
	}
}

fn get_val_naive_date(value: &Value) -> Result<Option<NaiveDate>> {
	match value {
		Value::String(s) => Ok(Some(NaiveDate::parse_from_str(s, "%Y-%m-%d")?)),
		_ => Ok(None),
	}
}

fn traverse_meta(meta: &MetadataRevision) -> Result<TempMeta> {
	let mut temp_meta = TempMeta::default();

	let tags = meta.tags();
	if tags.is_empty() {
		return Err(Error::descriptive("Tags were empty for this file"));
	}

	for tag in tags {
		// println!("{:?} ({}): {:#?}", tag.std_key, tag.key, tag.value);

		if let Some(std_key) = tag.std_key {
			match std_key {
				StandardTagKey::TrackTitle => {
					temp_meta.title = get_val_string(&tag.value);
				}
				StandardTagKey::Artist => {
					temp_meta.artist = get_val_string(&tag.value);
				}
				StandardTagKey::Album => {
					temp_meta.release = get_val_string(&tag.value);
				}
				StandardTagKey::AlbumArtist => {
					if let Some(x) = get_val_string(&tag.value) {
						temp_meta.album_artists.push(x);
					}
				}
				StandardTagKey::SortAlbumArtist => {
					if let Some(x) = get_val_string(&tag.value) {
						temp_meta.album_artist_sort.push(x);
					}
				}
				StandardTagKey::Composer => {
					if let Some(x) = get_val_string(&tag.value) {
						temp_meta.composers.push(x);
					}
				}
				StandardTagKey::Producer => {
					if let Some(x) = get_val_string(&tag.value) {
						temp_meta.producers.push(x);
					}
				}
				StandardTagKey::Label => {
					temp_meta.label = get_val_string(&tag.value);
				}
				StandardTagKey::IdentCatalogNumber => {
					temp_meta.catalog_number = get_val_string(&tag.value);
				}
				StandardTagKey::Date => match tag.key.as_str() {
					"DATE" => {
						temp_meta.date = get_val_naive_date(&tag.value)?;
					}
					"YEAR" => {
						temp_meta.year = get_val_str_or_u32(&tag.value)?;
					}
					_ => {}
				},
				StandardTagKey::OriginalDate => {
					temp_meta.original_date = get_val_naive_date(&tag.value)?;
				}
				StandardTagKey::DiscNumber => {
					temp_meta.disc_number = get_val_str_or_u32(&tag.value)?;
				}
				StandardTagKey::DiscTotal => {
					temp_meta.total_discs = get_val_str_or_u32(&tag.value)?;
				}
				StandardTagKey::TrackNumber => {
					temp_meta.track_number = get_val_str_or_u32(&tag.value)?;
				}
				StandardTagKey::TrackTotal => {
					temp_meta.total_tracks = get_val_str_or_u32(&tag.value)?;
				}
				StandardTagKey::Genre => {
					if let Some(x) = get_val_string(&tag.value) {
						temp_meta.genres.push(x);
					}
				}
				_ => {}
			}
		}

		match tag.key.as_str() {
			"ARTISTS" => {
				if let Some(x) = get_val_string(&tag.value) {
					temp_meta.artists.push(x);
				}
			}
			"ORIGINALYEAR" => {
				temp_meta.original_year = get_val_str_or_u32(&tag.value)?;
			}
			"RELEASETYPE" => {
				temp_meta.release_type = ReleaseType::from_str(&tag.value.to_string());
			}
			_ => {}
		}
	}

	Ok(temp_meta)
}

#[cfg(test)]
mod test {
	use std::fs::File;
	use std::path::Path;

	use crate::utils::symphonia::read_track_meta;

	#[test]
	fn test_read_track_meta() {
		let path =
			Path::new(r"c:\Users\Curstantine\Music\TempLib\Oh Shu & BIOMAN\Villa Tereze\10 Aeroporto di Bologna.flac");
		let file = File::open(path).unwrap();
		let extension = path.extension().and_then(|s| s.to_str());

		if let Err(e) = read_track_meta(Box::new(file), extension) {
			panic!("{:#?}", e);
		}
	}
}
