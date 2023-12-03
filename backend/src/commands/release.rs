use std::collections::{HashMap, HashSet};

use bonsaidb::core::{
	document::DocumentId,
	schema::{SerializedCollection, SerializedView},
};

use crate::{
	database::{
		models::{person::Person, release::Release, resource::Resource},
		views::release::ReleaseByNameAndArtist,
	},
	errors::Result,
	models::{
		state::AppState,
		tauri::release::{DisplayReleases, ReleaseEntity},
	},
};

#[tauri::command]
#[tracing::instrument(skip(app_state), err(Debug))]
pub async fn get_releases(library_id: u64, app_state: tauri::State<'_, AppState>) -> Result<Vec<ReleaseEntity>> {
	let db_lock = app_state.db.lock().await;
	let database = db_lock.as_ref().unwrap();
	let database = &database.0;

	let entries = ReleaseByNameAndArtist::entries_async(database)
		.query_with_docs()
		.await?;
	let mut releases = Vec::with_capacity(entries.len());

	for mapping in &entries {
		let id = mapping.document.header.id.deserialize::<u64>()?;
		let release = Release::document_contents(mapping.document)?;
		releases.push(ReleaseEntity::new(id, release))
	}

	Ok(releases)
}

#[tauri::command]
#[tracing::instrument(skip(app_state), err(Debug))]
pub async fn get_display_releases(library_id: u64, app_state: tauri::State<'_, AppState>) -> Result<DisplayReleases> {
	let db_lock = app_state.db.lock().await;
	let database = db_lock.as_ref().unwrap();
	let database = &database.0;

	let entries = ReleaseByNameAndArtist::entries_async(database)
		.query_with_docs()
		.await?;

	let mut releases = HashMap::with_capacity(entries.len());
	let mut artist_set = HashSet::<u64>::new();
	let mut cover_set = HashSet::<u64>::new();

	for mapping in &entries {
		let id = mapping.document.header.id.deserialize::<u64>()?;
		let release = Release::document_contents(mapping.document)?;

		release.artists.iter().for_each(|e| {
			artist_set.insert(e.id);
		});

		if let Some(covers) = &release.cover_ids {
			covers.iter().for_each(|e| {
				cover_set.insert(*e);
			});
		}

		releases.insert(id, release);
	}

	let artist_ids = artist_set.into_iter().map(DocumentId::from_u64).collect::<Vec<_>>();
	let cover_ids = cover_set.into_iter().map(DocumentId::from_u64).collect::<Vec<_>>();

	let mut artists = HashMap::<u64, Person>::with_capacity(artist_ids.len());
	let mut covers = HashMap::<u64, Resource>::with_capacity(cover_ids.len());

	for i in Person::get_multiple_async(&artist_ids, database).await? {
		artists.insert(i.header.id, i.contents);
	}

	for i in Resource::get_multiple_async(&cover_ids, database).await? {
		covers.insert(i.header.id, i.contents);
	}

	Ok(DisplayReleases {
		releases,
		artists,
		covers,
	})
}
