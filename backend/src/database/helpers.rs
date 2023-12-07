use std::{borrow::Cow, fs::File, path::Path};

use bonsaidb::{core::schema::SerializedCollection, local::AsyncDatabase};
use image::{imageops::FilterType, load_from_memory_with_format, ImageFormat};

use crate::{
	errors::{Error, FromErrorWithContextData, Result},
	models::temp::{resource::TempResource, TempTrackMeta, TempTrackResource},
};

use super::{
	methods,
	models::{
		resource::{Resource, ResourceType},
		InlinedArtist,
	},
};

/// Initializes an image resource and inserts the resource into the database, checking if the resource by same hash exists.
pub async fn initialize_image_resource(
	database: &AsyncDatabase,
	resource_cover_dir: &Path,
	temp: TempResource,
) -> Result<u64> {
	if temp.type_ != ResourceType::Image {
		let context = format!("Expected resource type to be `Image` but got {:?}", temp.type_);
		return Err(Error::descriptive("Invalid resource type").with_context(Cow::Owned(context)));
	}

	let hash = blake3::hash(&temp.data);
	let hash_str = hash.to_hex().to_string();
	let ext = temp.media_type.to_extension();

	if let Some(id) = methods::resource::get_id(database, temp.type_, temp.relation_type, hash).await? {
		return Ok(id);
	};

	let source_res_path = resource_cover_dir.join(format!("{}.{}", &hash_str, &ext));
	let thumb_res_path = resource_cover_dir.join(format!("{}@512.{}", &hash_str, &ext));

	let handle = tokio::task::spawn_blocking::<_, Result<Resource>>(move || {
		std::fs::write(&source_res_path, &temp.data).map_err(|e| Error::from_std_path(e, &source_res_path))?;

		let fmt = ImageFormat::from_extension(ext).expect("Unsupported file extension");
		let resizable = load_from_memory_with_format(&temp.data, fmt)
			.map_err(|e| Error::from_with_ctx(e, &thumb_res_path))?
			.resize(512, 512, FilterType::Nearest);

		let mut file = File::create(&thumb_res_path).map_err(|e| Error::from_std_path(e, &thumb_res_path))?;
		resizable
			.write_to(&mut file, fmt)
			.map_err(|e| Error::from_with_ctx(e, &thumb_res_path))?;

		Ok(temp.into_resource(hash))
	});

	let doc = handle.await??.push_into_async(database).await?;
	Ok(doc.header.id)
}

/// Deduplicates and inserts a track with its metadata.
pub async fn handle_temp_track_meta(
	database: &AsyncDatabase,
	resource_cover_dir: &Path,
	meta: TempTrackMeta,
	resource: TempTrackResource,
) -> Result<()> {
	let temp_track = match meta.track {
		Some(x) => x,
		None => return Err(Error::descriptive("No track metadata found")),
	};

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
			let res = initialize_image_resource(database, resource_cover_dir, temp).await?;
			x.push(res);
		}
	}

	if let Some(track_covers) = resource.track_covers {
		let x = track_cover_ids.insert(Vec::with_capacity(track_covers.len()));

		for temp in track_covers {
			let res = initialize_image_resource(database, resource_cover_dir, temp).await?;
			x.push(res);
		}
	}

	if let Some(temp) = meta.release {
		let release = temp.into_release(
			release_artists,
			label_ids,
			genre_ids.clone(),
			tag_ids.clone(),
			release_cover_ids,
		);

		let id = methods::release::get_or_insert(database, release).await?;
		release_id = Some(id);
	}

	temp_track
		.into_track(
			artists,
			release_id,
			composer_ids,
			producer_ids,
			genre_ids,
			tag_ids,
			track_cover_ids,
		)
		.push_into_async(database)
		.await?;

	Ok(())
}
