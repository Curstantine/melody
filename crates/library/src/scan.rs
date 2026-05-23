use std::ffi::CString;

use anyhow::{Context, Result, anyhow, bail};
use chrono::NaiveDate;
use rsmpeg::{
	avformat::AVFormatContextInput,
	avutil::AVDictionaryRef,
	ffi::{AV_DISPOSITION_ATTACHED_PIC, AVMEDIA_TYPE_AUDIO, AVMEDIA_TYPE_VIDEO},
};

use db::models::{
	CountryCode, FromTag, ScriptCode,
	cover::{CoverMediaType, CoverType},
	label::Label,
	person::{Person, PersonType},
	release::{ReleaseType, ReleaseTypeSecondary},
	tag::{Tag, TagType},
	temp::{OptionedDate, TempTrackMeta, TempTrackResource, cover::TempCover, person::TempInlinePerson},
};

pub fn read_track_meta(path: String) -> Result<(TempTrackMeta, TempTrackResource)> {
	let path_cstr = CString::new(path.as_bytes())?;
	let format = AVFormatContextInput::open(&path_cstr)?;

	let tags = if let Some(meta) = format.metadata() {
		traverse_tags(meta, path)?
	} else if let Some((index, _)) = format.find_best_stream(AVMEDIA_TYPE_AUDIO)? {
		let stream = format
			.streams()
			.get(index)
			.context("Failed to find best audio stream")?;
		let meta = stream
			.metadata()
			.ok_or_else(|| anyhow!("No metadata for stream: {:?}", path))?;
		traverse_tags(meta, path)?
	} else {
		bail!("Probing led to no data: {:?}", path);
	};

	let mut resource = TempTrackResource::default();
	if let Some((index, _)) = format.find_best_stream(AVMEDIA_TYPE_VIDEO)? {
		let stream = format
			.streams()
			.get(index)
			.context("Failed to find best video stream")?;

		if stream.disposition as u32 & AV_DISPOSITION_ATTACHED_PIC != 0 {
			let pic = stream.attached_pic;
			let codec = stream.codecpar();
			let opt = resource.release_covers.get_or_insert_with(Vec::new);

			let comment = if let Some(meta) = stream.metadata() {
				let key = CString::new("comment")?;
				let h = meta.get(key.as_c_str(), None, 0);
				h.map(|x| x.value().to_string_lossy().to_string())
			} else {
				None
			};

			if pic.data.is_null() || pic.size <= 0 {
				bail!("Picture has invalid data: ptr={:?}, size={}", pic.data, pic.size);
			}

			// pic.data is verified non-null and pic.size is verified > 0 above
			let data = unsafe {
				let slice = std::slice::from_raw_parts(pic.data, pic.size as usize);
				Box::from(slice)
			};

			opt.push(TempCover {
				type_: CoverType::Release,
				media_type: CoverMediaType::from_codec_id(codec.codec_id)?,
				resolution: (codec.width as u16, codec.height as u16),
				comment,
				data,
			});
		}
	}

	Ok((tags, resource))
}

fn traverse_tags(dict: AVDictionaryRef<'_>, path_str: String) -> Result<TempTrackMeta> {
	let mut meta = TempTrackMeta { ..Default::default() };
	let mut mbz_artist_ids: Option<Vec<String>> = None;

	meta.get_or_default_track().path = path_str;

	for tag in dict.into_iter() {
		let key = tag.key().to_str().unwrap().to_lowercase();
		let val = tag.value().to_string_lossy().to_string();

		#[cfg(test)]
		println!("key: {:?}, value: {:#?}", key, val);

		match key.as_str() {
			"title" => {
				let x = meta.get_or_default_track();
				x.title = val;
			}
			"title_sort" | "titlesort" => {
				let x = meta.get_or_default_track();
				x.title_sort = Some(val);
			}

			"artist" | "composer" | "producer" => {
				let x = meta.artists.get_or_insert_with(Vec::new);
				x.extend(val.split(';').map(str::trim).filter(|s| !s.is_empty()).map(|s| {
					let y = Person::temp(s.to_string(), None, None);
					TempInlinePerson::new(y, PersonType::from_tag(&key))
				}));
			}
			"artist_sort" | "artistsort" => {
				let x = meta.get_or_default_track();
				x.artist_sort = Some(val);
			}

			"artists" => {
				let x = meta.weak_artists.get_or_insert_with(Vec::new);
				x.extend(val.split(';').map(str::trim).filter(|s| !s.is_empty()).map(|s| {
					let y = Person::temp(s.to_string(), None, None);
					TempInlinePerson::new(y, PersonType::Artist)
				}));
			}

			"album" => {
				let x = meta.get_or_default_release();
				x.name = val;
			}
			"album_sort" | "albumsort" => {
				let x = meta.get_or_default_release();
				x.name_sort = Some(val);
			}
			"album_artist" | "albumartist" => {
				let x = meta.release_artists.get_or_insert_with(Vec::new);
				let y = Person::temp(val, None, None);
				x.push(TempInlinePerson::new(y, PersonType::Artist))
			}
			"album_artist_sort" | "albumartistsort" => {
				let x = meta.get_or_default_release();
				x.artist_sort = Some(val);
			}

			"script" => {
				let x = meta.get_or_default_release();
				x.script = Some(ScriptCode::from_tag(&val))
			}
			"release_country" | "releasecountry" => {
				let x = meta.get_or_default_release();
				x.country = Some(CountryCode::from_tag(&val));
			}

			"track" => {
				if let Ok(Some((track_no, track_total_opt))) = get_no_and_maybe_total(&val) {
					let y = meta.get_or_default_track();
					y.track_number = Some(track_no);

					if let Some(track_total) = track_total_opt {
						let z = meta.get_or_default_release();
						z.total_tracks.get_or_insert(track_total);
					}
				}
			}
			"disc" => {
				if let Ok(Some((disc_no, disc_total_opt))) = get_no_and_maybe_total(&val) {
					let y = meta.get_or_default_track();
					y.disc_number = Some(disc_no);

					if let Some(disc_total) = disc_total_opt {
						let z = meta.get_or_default_release();
						z.total_discs.get_or_insert(disc_total);
					}
				}
			}

			"total_tracks" | "totaltracks" => {
				if let Ok(y) = val.parse::<u32>() {
					let x = meta.get_or_default_release();
					x.total_tracks = Some(y);
				}
			}
			"total_discs" | "totaldiscs" => {
				if let Ok(y) = val.parse::<u32>() {
					let x = meta.get_or_default_release();
					x.total_discs = Some(y);
				}
			}

			"original_date" | "originaldate" => {
				if let Ok(Some((Some(year), Some(month), day_opt))) = get_val_date(&val) {
					let y = meta.get_or_default_track();
					y.original_date = NaiveDate::from_ymd_opt(year, month, day_opt.unwrap_or(1));
				}
			}
			"date" => match get_val_date(&val) {
				Ok(Some((Some(year), Some(month), day_opt))) => {
					let y = meta.get_or_default_release();
					y.date = NaiveDate::from_ymd_opt(year, month, day_opt.unwrap_or(1));
				}
				Ok(Some((Some(year), None, None))) => {
					let y = meta.get_or_default_release();
					if y.year.is_none() {
						y.year = Some(year);
					}
				}
				_ => {}
			},

			"label" => {
				let x = meta.labels.get_or_insert_with(Vec::new);
				let y = Label::temp(val);
				x.push(y);
			}
			"catalog" | "catalognumber" => {
				let x = meta.get_or_default_release();
				x.catalog_number = Some(val);
			}

			"genre" => {
				let x = meta.tags.get_or_insert_with(Vec::new);
				x.extend(
					val.split(&[';', ','])
						.map(str::trim)
						.filter(|s| !s.is_empty())
						.map(|s| Tag::temp(s.to_string(), TagType::Genre)),
				);
			}

			"musicbrainz_trackid" => {
				let x = meta.get_or_default_track();
				x.mbz_id = Some(val);
			}
			"musicbrainz_albumid" => {
				let x = meta.get_or_default_release();
				x.mbz_id = Some(val);
			}
			"musicbrainz_artistid" => {
				mbz_artist_ids = Some(
					val.split(';')
						.map(str::trim)
						.filter(|s| !s.is_empty())
						.map(|s| s.to_owned())
						.collect(),
				);
			}
			"releasetype" => {
				let x = meta.get_or_default_release();
				let mut parts = val.split(';');

				if let Some(y) = parts.next().map(ReleaseType::from_tag) {
					x.type_ = y;
				}

				if let Some(y) = parts.next().map(ReleaseTypeSecondary::from_tag) {
					x.type_secondary.get_or_insert_with(Vec::new).push(y);
				}
			}

			_ => continue,
		}
	}

	if let Some((arts, ids)) = meta.weak_artists.as_mut().zip(mbz_artist_ids) {
		for (art, id) in arts.iter_mut().zip(ids.into_iter()) {
			if !id.is_empty() {
				art.person.mbz_id = Some(id.to_owned());
			}
		}
	}

	Ok(meta)
}

#[inline]
fn get_val_date(x: &str) -> Result<OptionedDate> {
	let normalized_date = x.replace(['.', '/'], "-");
	let mut parts = normalized_date.split('-');

	let date = match (parts.next(), parts.next(), parts.next()) {
		(Some(y), Some(m), Some(d)) => Some((
			Some(y.parse::<i32>()?),
			Some(m.parse::<u32>()?),
			Some(d.parse::<u32>()?),
		)),
		(Some(y), Some(m), None) => Some((Some(y.parse::<i32>()?), Some(m.parse::<u32>()?), None)),
		(Some(y), None, None) => Some((Some(y.parse::<i32>()?), None, None)),
		_ => None,
	};

	Ok(date)
}

/// Reads into a value and tries to get an int followed by an optional int separated by a forward slash.
///
/// Useful for handling edge cases like track_no and track_total included in the same tag.
///
/// ### Example
/// "2" -> (2, None)
/// "1/2" -> (1, 2)
#[inline]
fn get_no_and_maybe_total(value: &str) -> Result<Option<(u32, Option<u32>)>> {
	let mut parts = value.split('/');
	let tuple = match (parts.next(), parts.next()) {
		(Some(no), Some(total)) => Some((no.parse::<u32>()?, Some(total.parse::<u32>()?))),
		(Some(no), None) => Some((no.parse::<u32>()?, None)),
		_ => None,
	};

	Ok(tuple)
}

#[cfg(test)]
mod test {
	use anyhow::Result;

	use super::{get_no_and_maybe_total, get_val_date, read_track_meta};

	// const TRACK_PATH: &str =
	// 	r"/home/curstantine/Music/TempLib/Various Artists/IRREGULAR NATION/01 Massive New Krew - MUTANT.flac";
	const TRACK_PATH: &str = r"/home/curstantine/Music/Library/青葉市子/海底のエデン/01 海底のエデン.flac";
	// const TRACK_PATH: &str =
	// 	r"/home/curstantine/Music/Library/Mili feat. KIHOW/In Hell We Live, Lament/01 In Hell We Live, Lament.opus";

	#[test]
	fn test_read_track_meta() -> Result<()> {
		let (meta, resource) = read_track_meta(TRACK_PATH.into())?;
		println!("{:#?}", meta);
		println!("{:#?}", resource);

		Ok(())
	}

	#[test]
	fn test_get_no_and_maybe_total() -> Result<()> {
		// Single number
		assert_eq!(get_no_and_maybe_total("2")?, Some((2, None)));
		assert_eq!(get_no_and_maybe_total("0")?, Some((0, None)));
		assert_eq!(get_no_and_maybe_total("42")?, Some((42, None)));

		// Number with total
		assert_eq!(get_no_and_maybe_total("1/2")?, Some((1, Some(2))));
		assert_eq!(get_no_and_maybe_total("3/10")?, Some((3, Some(10))));
		assert_eq!(get_no_and_maybe_total("0/0")?, Some((0, Some(0))));

		// Non-numeric should error
		assert!(get_no_and_maybe_total("abc").is_err());
		assert!(get_no_and_maybe_total("abc/def").is_err());
		assert!(get_no_and_maybe_total("").is_err());

		Ok(())
	}

	#[test]
	fn test_get_val_date() -> Result<()> {
		// Full date (year, month, day)
		assert_eq!(get_val_date("2024-01-15")?, Some((Some(2024), Some(1), Some(15))));

		// Year and month only
		assert_eq!(get_val_date("2024-01")?, Some((Some(2024), Some(1), None)));

		// Year only
		assert_eq!(get_val_date("2024")?, Some((Some(2024), None, None)));

		// Non-date garbage
		assert!(get_val_date("not-a-date").is_err());
		assert!(get_val_date("abc-def").is_err());
		assert!(get_val_date("").is_err());

		Ok(())
	}
}
