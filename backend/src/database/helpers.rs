use std::{fs, path::Path};

use {
	bonsaidb::{core::schema::SerializedCollection, local::AsyncDatabase},
	image::{imageops::FilterType, ImageFormat},
};

use crate::{
	database::{
		methods,
		models::{cover::Cover, InlinedArtist},
	},
	errors::{pre::unsupported_image_type, Result},
	models::{
		directories,
		temp::{
			cover::TempCover, release::TempReleaseIntoArg, track::TempTrackIntoArg, TempTrackMeta, TempTrackResource,
		},
	},
};

/// Initializes an image resource and inserts the resource into the database, checking if the resource by same hash exists.
pub async fn initialize_image_resource(database: &AsyncDatabase, cover_dir: &Path, temp: TempCover) -> Result<u64> {
	let hash = blake3::hash(&temp.data);

	if let Some(id) = methods::cover::get_id(database, temp.type_, hash).await? {
		return Ok(id);
	};

	let needs_thumb = temp.needs_thumb();
	let extension = temp.media_type.as_extension();
	let path = directories::get_cover_path(cover_dir, &hash, extension, needs_thumb);

	let cover = tokio::task::spawn_blocking::<_, Result<Cover>>(move || {
		if needs_thumb {
			let fmt = ImageFormat::from_extension(extension).ok_or_else(|| unsupported_image_type(extension))?;
			let source = image::load_from_memory_with_format(&temp.data, fmt)?;

			source
				.resize(512, 512, FilterType::Nearest)
				.save_with_format(path, ImageFormat::Png)?;
		} else {
			fs::write(path, &temp.data)?;
		}

		Ok(temp.into_cover(hash, needs_thumb))
	})
	.await??;

	let doc = cover.push_into_async(database).await?;

	Ok(doc.header.id)
}

/// Deduplicates and inserts a track with its metadata.
pub async fn handle_temp_track_meta(
	database: &AsyncDatabase,
	cover_dir: &Path,
	meta: TempTrackMeta,
	resource: TempTrackResource,
) -> Result<()> {
	let temp_track = meta.track.expect("Yeah, no track metadata.");

	let mut artists = None::<Vec<InlinedArtist>>;
	let mut composer_ids = None::<Vec<u64>>;
	let mut producer_ids = None::<Vec<u64>>;

	let mut label_ids = None::<Vec<u64>>;
	let mut genre_ids = None::<Vec<u64>>;
	let mut tag_ids = None::<Vec<u64>>;
	let mut release_artists = None::<Vec<InlinedArtist>>;

	let mut release_cover_ids = None::<Vec<u64>>;
	let mut track_cover_ids = None::<Vec<u64>>;

	let mut release_id = None::<u64>;

	if let Some(temp_artists) = meta.artists {
		let x = artists.get_or_insert(Vec::with_capacity(temp_artists.len()));

		for temp_artist in temp_artists {
			let id = methods::person::get_or_insert(database, temp_artist.person.clone()).await?;
			x.push(temp_artist.into_inlined(id));
		}
	}

	if let Some(temp_composers) = meta.composers {
		let x = composer_ids.get_or_insert(Vec::with_capacity(temp_composers.len()));

		for temp_composer in temp_composers {
			let id = methods::person::get_or_insert(database, temp_composer).await?;
			x.push(id);
		}
	}

	if let Some(temp_producers) = meta.producers {
		let x = producer_ids.get_or_insert(Vec::with_capacity(temp_producers.len()));

		for temp_producer in temp_producers {
			let id = methods::person::get_or_insert(database, temp_producer).await?;
			x.push(id);
		}
	}

	if let Some(temp_labels) = meta.labels {
		let x = label_ids.get_or_insert(Vec::with_capacity(temp_labels.len()));

		for temp_label in temp_labels {
			let id = methods::label::get_or_insert(database, temp_label).await?;
			x.push(id);
		}
	}

	if let Some(temp_genres) = meta.genres {
		let x = genre_ids.get_or_insert(Vec::with_capacity(temp_genres.len()));

		for temp_genre in temp_genres {
			let id = methods::tag::get_or_insert(database, temp_genre).await?;
			x.push(id);
		}
	}

	if let Some(temp_tags) = meta.tags {
		let x = tag_ids.get_or_insert(Vec::with_capacity(temp_tags.len()));

		for temp_tag in temp_tags {
			let id = methods::tag::get_or_insert(database, temp_tag).await?;
			x.push(id);
		}
	}

	if let Some(temp_release_artists) = meta.release_artists {
		let y = release_artists.get_or_insert(Vec::with_capacity(temp_release_artists.len()));

		for temp_artist in temp_release_artists {
			let id = methods::person::get_or_insert(database, temp_artist.person.clone()).await?;
			y.push(temp_artist.into_inlined(id));
		}
	}

	if let Some(release_covers) = resource.release_covers {
		let x = release_cover_ids.insert(Vec::with_capacity(release_covers.len()));

		for temp in release_covers {
			let res = initialize_image_resource(database, cover_dir, temp).await?;
			x.push(res);
		}
	}

	if let Some(track_covers) = resource.track_covers {
		let x = track_cover_ids.insert(Vec::with_capacity(track_covers.len()));

		for temp in track_covers {
			let res = initialize_image_resource(database, cover_dir, temp).await?;
			x.push(res);
		}
	}

	if let Some(temp) = meta.release {
		let release = temp.into_release(TempReleaseIntoArg {
			artists: release_artists,
			label_ids,
			genre_ids: genre_ids.clone(),
			tag_ids: tag_ids.clone(),
			cover_ids: release_cover_ids,
		});

		let id = methods::release::get_or_insert(database, release).await?;
		release_id = Some(id);
	}

	temp_track
		.into_track(TempTrackIntoArg {
			artists,
			release_id,
			composer_ids,
			producer_ids,
			genre_ids,
			tag_ids,
			cover_ids: track_cover_ids,
		})
		.push_into_async(database)
		.await?;

	Ok(())
}
