use std::{ffi::CString, path::Path};

use {
	chrono::NaiveDate,
	rsmpeg::{
		avformat::AVFormatContextInput,
		avutil::AVDictionaryRef,
		ffi::{AVMediaType_AVMEDIA_TYPE_AUDIO, AVMediaType_AVMEDIA_TYPE_VIDEO, AV_DISPOSITION_ATTACHED_PIC},
	},
};

use crate::{
	database::models::{
		cover::{CoverMediaType, CoverType},
		person::PersonType,
		release::{ReleaseType, ReleaseTypeSecondary},
		tag::TagType,
		CountryCode, FromTag, ScriptCode,
	},
	errors::{self, Result},
	models::temp::{
		cover::TempCover, label::TempLabel, person::TempPerson, tag::TempTag, OptionedDate, TempPersonCredit,
		TempTrackMeta, TempTrackResource,
	},
	utils::matchers,
};

pub fn read_track_meta(path: &Path) -> Result<(TempTrackMeta, TempTrackResource)> {
	let path_str = path.to_str().unwrap().to_string();
	let path_cstr = CString::new(path_str.as_bytes()).unwrap();

	#[allow(unused_mut)]
	let mut format = AVFormatContextInput::open(&path_cstr, None, &mut None)?;

	#[cfg(test)]
	format.dump(0, &path_cstr)?;

	let tags = if let Some(meta) = format.metadata() {
		traverse_tags(meta, path_str)?
	} else if let Some((index, _)) = format.find_best_stream(AVMediaType_AVMEDIA_TYPE_AUDIO)? {
		let stream = format.streams().get(index).unwrap();
		let meta = stream.metadata().ok_or_else(errors::pre::probe_no_meta)?;

		traverse_tags(meta, path_str)?
	} else {
		return Err(errors::pre::probe_no_meta());
	};

	let mut resource = TempTrackResource::default();
	if let Some((index, _)) = format.find_best_stream(AVMediaType_AVMEDIA_TYPE_VIDEO)? {
		let stream = format.streams().get(index).unwrap();

		if stream.disposition as u32 == AV_DISPOSITION_ATTACHED_PIC {
			let pic = stream.attached_pic;
			let codec = stream.codecpar();
			let opt = resource.release_covers.get_or_insert_with(Vec::new);

			let comment = if let Some(meta) = stream.metadata() {
				let key = CString::new("comment").unwrap();
				let h = meta.get(key.as_c_str(), None, 0);
				h.map(|x| x.value().to_string_lossy().to_string())
			} else {
				None
			};

			// We will have to copy the slice into a vec regardless because we don't own the
			// memory from libavcodec, and I feel safer this way.
			let data = unsafe {
				let slice = std::slice::from_raw_parts(pic.data, pic.size as usize);
				slice.to_vec().into_boxed_slice()
			};

			opt.push(TempCover {
				type_: CoverType::Release,
				media_type: CoverMediaType::from_codec_id(codec.codec_id)?,
				resolution: (codec.height as u16, codec.width as u16),
				comment,
				data,
			});
		}
	}

	Ok((tags, resource))
}

fn traverse_tags(dict: AVDictionaryRef<'_>, path_str: String) -> Result<TempTrackMeta> {
	let mut meta = TempTrackMeta {
		path: path_str,
		..Default::default()
	};

	let mut used_artists_field = false;
	let mut primary_release_type_used = false;

	for tag in dict.into_iter() {
		let key = tag.key().to_str().unwrap().to_lowercase();
		let val = tag.value().to_string_lossy().to_string();

		match key.as_str() {
			"title" => {
				let x = meta.get_or_default_track();
				x.title = val;
			}
			"title_sort" | "titlesort" => {
				let x = meta.get_or_default_track();
				x.title_sort = Some(val);
			}

			"artist" if !used_artists_field => {
				let x = meta.artists.get_or_insert_with(Vec::new);
				let y = TempPerson {
					name: val,
					type_: PersonType::Artist,
					name_sort: None,
					mbz_id: None,
				};

				x.push(TempPersonCredit::from(y))
			}
			"artist_sort" | "artistsort" => {
				let x = meta.get_or_default_track();
				x.artist_sort = Some(val);
			}
			"composer" => {
				let x = meta.composers.get_or_insert_with(Vec::new);
				let y = TempPerson {
					name: val,
					type_: PersonType::Composer,
					name_sort: None,
					mbz_id: None,
				};

				x.push(y)
			}
			"producer" => {
				let x = meta.producers.get_or_insert_with(Vec::new);
				let y = TempPerson {
					name: val,
					type_: PersonType::Producer,
					name_sort: None,
					mbz_id: None,
				};

				x.push(y)
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
				let y = TempPerson {
					name: val,
					type_: PersonType::Artist,
					name_sort: None,
					mbz_id: None,
				};

				x.push(TempPersonCredit::from(y))
			}
			"album_artist_sort" | "albumartistsort" => {
				let x = meta.get_or_default_release();
				x.artist_sort = Some(val);
			}

			"script" => {
				let x = meta.get_or_default_release();
				let y = ScriptCode::from_tag(val.as_str()).unwrap();
				x.script = Some(y);
			}
			"release_country" | "releasecountry" => {
				let x = meta.get_or_default_release();
				let y = CountryCode::from_tag(val.as_str()).unwrap();
				x.country = Some(y);
			}

			"track" => {
				if let Some((track_no, track_total_opt)) = get_no_and_maybe_total(val)? {
					let y = meta.get_or_default_track();
					y.track_number = Some(track_no);

					if let Some(track_total) = track_total_opt {
						let z = meta.get_or_default_release();
						z.total_tracks.get_or_insert(track_total);
					}
				}
			}
			"disc" => {
				if let Some((disc_no, disc_total_opt)) = get_no_and_maybe_total(val)? {
					let y = meta.get_or_default_track();
					y.disc_number = Some(disc_no);

					if let Some(disc_total) = disc_total_opt {
						let z = meta.get_or_default_release();
						z.total_discs.get_or_insert(disc_total);
					}
				}
			}

			"total_tracks" | "totaltracks" => {
				let y = val.parse::<u32>()?;
				let x = meta.get_or_default_release();
				x.total_tracks = Some(y);
			}
			"total_discs" | "totaldiscs" => {
				let y = val.parse::<u32>()?;
				let x = meta.get_or_default_release();
				x.total_discs = Some(y);
			}

			"original_date" | "originaldate" => {
				if let Some((Some(year), Some(month), day_opt)) = get_val_date(val)? {
					let y = meta.get_or_default_track();
					y.original_date = NaiveDate::from_ymd_opt(year, month, day_opt.unwrap_or(1));
				}
			}
			"date" => match get_val_date(val)? {
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

			"label" => {
				let x = meta.labels.get_or_insert_with(Vec::new);
				let y = TempLabel { name: val };
				x.push(y);
			}
			"catalog" | "catalognumber" => {
				let x = meta.get_or_default_release();
				x.catalog_number = Some(val);
			}

			"genre" => {
				let x = meta.genres.get_or_insert_with(Vec::new);
				let y = TempTag {
					name: val,
					type_: TagType::Genre,
				};

				x.push(y);
			}

			"musicbrainz_trackid" => {
				let x = meta.get_or_default_track();
				x.mbz_id = Some(val);
			}
			"musicbrainz_albumid" => {
				let x = meta.get_or_default_release();
				x.mbz_id = Some(val);
			}

			"artists" => {
				let y = TempPersonCredit::from(TempPerson {
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

			"RELEASETYPE" if !primary_release_type_used => {
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
			"RELEASETYPE" if primary_release_type_used => {
				let x = meta.get_or_default_release();
				let y = ReleaseTypeSecondary::from_tag(val.as_str()).unwrap();
				x.type_secondary.get_or_insert_with(Vec::new).push(y);
			}

			_ => continue,
		}
	}

	Ok(meta)
}

#[inline]
fn get_val_date(x: String) -> Result<OptionedDate> {
	let date: OptionedDate = if matchers::reg::is_ymd(x.as_str()) {
		let splits = x.split('-').collect::<Vec<&str>>();

		let year = {
			let y = splits.first().unwrap();
			y.parse::<i32>()?
		};
		let month = {
			let y = splits.get(1).unwrap();
			y.parse::<u32>()?
		};
		let day = {
			let y = splits.get(2).unwrap();
			y.parse::<u32>()?
		};

		Some((Some(year), Some(month), Some(day)))
	} else if matchers::reg::is_ym(x.as_str()) {
		let splits = x.split('-').collect::<Vec<&str>>();

		let year = {
			let y = splits.first().unwrap();
			y.parse::<i32>()?
		};
		let month = {
			let y = splits.get(1).unwrap();
			y.parse::<u32>()?
		};

		Some((Some(year), Some(month), None))
	} else if matchers::reg::is_year(x.as_str()) {
		let year = x.parse::<i32>()?;
		Some((Some(year), None, None))
	} else {
		None
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
fn get_no_and_maybe_total(value: String) -> Result<Option<(u32, Option<u32>)>> {
	let tuple = if matchers::reg::is_no_and_total(value.as_str()) {
		let splits = value.split('/').collect::<Vec<&str>>();
		let no_str = splits.first().unwrap();
		let total_str = splits.last().unwrap();

		let no = no_str.parse::<u32>()?;
		let total = total_str.parse::<u32>()?;

		Some((no, Some(total)))
	} else {
		Some((value.parse::<u32>()?, None))
	};

	Ok(tuple)
}

#[cfg(test)]
mod test {
	use std::path::Path;

	use super::read_track_meta;
	use crate::errors::Result;

	const TRACK_PATH: &str = r"C:\\Users\\Curstantine\\Music\\TempLib\\青葉市子\\海底のエデン\\01 海底のエデン.flac";
	// const TRACK_PATH: &str = r"C:\Users\Curstantine\Music\TempLib\nowisee\reALIVE\01 明日地球が滅ぶなら.flac";

	#[test]
	fn test_read_track_meta() -> Result<()> {
		let path = Path::new(TRACK_PATH);
		let result = read_track_meta(path)?;
		println!("{:#?}", result);

		Ok(())
	}
}
