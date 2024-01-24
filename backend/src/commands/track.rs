use std::collections::{HashMap, HashSet};

use tracing::debug;

use {
	bonsaidb::core::{
		document::DocumentId,
		schema::{SerializedCollection, SerializedView},
	},
	tokio::time::Instant,
};

use crate::{
	database::{
		models::{person::Person, track::Track},
		views::track::TrackByReleaseId,
	},
	errors::Result,
	models::{state::DatabaseState, tauri::track::DisplayTrackList},
};

#[tauri::command]
#[tracing::instrument(skip(db_state), err(Debug))]
pub async fn get_track_list_for_release(
	release_id: u64,
	db_state: tauri::State<'_, DatabaseState>,
) -> Result<DisplayTrackList> {
	let start = Instant::now();

	let db_guard = db_state.get().await;
	let database = db_guard.as_ref().unwrap();

	let entries = TrackByReleaseId::entries_async(database.inner_ref())
		.with_key(&release_id)
		.query_with_collection_docs()
		.await?;

	let mut tracks = Vec::<Track>::with_capacity(entries.len());
	let mut artist_ids = HashSet::<DocumentId>::new();

	for (_, document) in entries.documents {
		for artist in &document.contents.artists {
			artist_ids.insert(DocumentId::from_u64(artist.id));
		}

		tracks.push(document.contents);
	}

	let mut artists = HashMap::<u64, Person>::with_capacity(artist_ids.len());
	for i in Person::get_multiple_async(&artist_ids, database.inner_ref()).await? {
		artists.insert(i.header.id, i.contents);
	}

	tracks.sort_by(|a, b| a.track_number.cmp(&b.track_number));
	debug!("Finished building display track list query in {:?}", start.elapsed());

	Ok(DisplayTrackList { tracks, artists })
}
