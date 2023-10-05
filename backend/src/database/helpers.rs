use bonsaidb::{core::schema::SerializedCollection, local::AsyncDatabase};

use crate::{
	errors::{Error, Result},
	models::temp::TempTrackMeta,
};

use super::{
	models::InlinedArtist,
	views::{label::LabelByName, person::PersonByNameAndType, release::ReleaseByNameAndArtist, tag::TagByNameAndType},
};

/// Deduplicates and inserts a track with its metadata.
pub async fn handle_temp_track_meta(database: &AsyncDatabase, meta: TempTrackMeta) -> Result<()> {
	let temp_track = match meta.track {
		Some(x) => x,
		None => return Err(Error::descriptive("No track metadata found")),
	};

	// NOTE:
	// We might not need to spawn tasks here,
	// since we could come across race conditions on which duplicated entry to put into the db, lol.
	let mut artists = None::<Vec<InlinedArtist>>;
	let mut composer_ids = None::<Vec<u64>>;
	let mut producer_ids = None::<Vec<u64>>;

	let mut label_ids = None::<Vec<u64>>;
	let mut genre_ids = None::<Vec<u64>>;
	let mut tag_ids = None::<Vec<u64>>;

	let mut release_id = None::<u64>;
	let mut release_artists = None::<Vec<InlinedArtist>>;

	if let Some(temp_artists) = meta.artists {
		let x = artists.get_or_insert(Vec::with_capacity(temp_artists.len()));

		for temp_artist in temp_artists {
			let id = PersonByNameAndType::put_or_get(database, temp_artist.person.clone()).await?;
			x.push(temp_artist.into_inlined(id));
		}
	}

	if let Some(temp_composers) = meta.composers {
		let x = composer_ids.get_or_insert(Vec::with_capacity(temp_composers.len()));

		for temp_composer in temp_composers {
			let id = PersonByNameAndType::put_or_get(database, temp_composer).await?;
			x.push(id);
		}
	}

	if let Some(temp_producers) = meta.producers {
		let x = producer_ids.get_or_insert(Vec::with_capacity(temp_producers.len()));

		for temp_producer in temp_producers {
			let id = PersonByNameAndType::put_or_get(database, temp_producer).await?;
			x.push(id);
		}
	}

	if let Some(temp_labels) = meta.labels {
		let x = label_ids.get_or_insert(Vec::with_capacity(temp_labels.len()));

		for temp_label in temp_labels {
			let id = LabelByName::put_or_get(database, temp_label).await?;
			x.push(id);
		}
	}

	if let Some(temp_genres) = meta.genres {
		let x = genre_ids.get_or_insert(Vec::with_capacity(temp_genres.len()));

		for temp_genre in temp_genres {
			let id = TagByNameAndType::put_or_get(database, temp_genre).await?;
			x.push(id);
		}
	}

	if let Some(temp_tags) = meta.tags {
		let x = tag_ids.get_or_insert(Vec::with_capacity(temp_tags.len()));

		for temp_tag in temp_tags {
			let id = TagByNameAndType::put_or_get(database, temp_tag).await?;
			x.push(id);
		}
	}

	if let Some(temp_release_artists) = meta.release_artists {
		let y = release_artists.get_or_insert(Vec::with_capacity(temp_release_artists.len()));

		for temp_artist in temp_release_artists {
			let id = PersonByNameAndType::put_or_get(database, temp_artist.person.clone()).await?;
			y.push(temp_artist.into_inlined(id));
		}
	}

	if let Some(temp) = meta.release {
		let release = temp.into_release(release_artists, label_ids, genre_ids.clone(), tag_ids.clone());
		release_id = Some(ReleaseByNameAndArtist::put_or_get(database, release).await?);
	}

	let track = temp_track.into_track(artists, release_id, composer_ids, producer_ids, genre_ids, tag_ids);
	track.push_into_async(database).await?;

	Ok(())
}
