use std::collections::{HashMap, HashSet};

use {
	bonsaidb::core::{
		document::DocumentId,
		schema::{SerializedCollection, SerializedView},
	},
	tauri::State,
	tokio::time::Instant,
	tracing::debug,
};

use crate::{
	database::{
		models::{cover::Cover, person::Person, release::Release},
		views::release::ReleaseByNameAndArtist,
	},
	errors::Result,
	models::{
		state::{DatabaseState, DirectoryState},
		tauri::{
			cover::DisplayCoverResource,
			release::{DisplayReleases, ReleaseEntity},
		},
	},
};

#[tauri::command]
#[tracing::instrument(skip(db_state), err(Debug))]
pub async fn get_releases(db_state: State<'_, DatabaseState>) -> Result<Vec<ReleaseEntity>> {
	let db_guard = db_state.get().await;
	let database = db_guard.as_ref().unwrap();

	let entries = ReleaseByNameAndArtist::entries_async(database.inner_ref())
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
#[tracing::instrument(skip(dir_state, db_state), err(Debug))]
pub async fn get_display_releases(
	dir_state: State<'_, DirectoryState>,
	db_state: State<'_, DatabaseState>,
) -> Result<DisplayReleases> {
	let start = Instant::now();

	let resource_cover_dir = {
		let dir_guard = dir_state.get();
		let directories = dir_guard.as_ref().unwrap();
		directories.cover_dir.clone()
	};

	let db_guard = db_state.get().await;
	let database = db_guard.as_ref().unwrap();

	let entries = ReleaseByNameAndArtist::entries_async(database.inner_ref())
		.query_with_docs()
		.await?;

	let mut releases = HashMap::<u64, Release>::with_capacity(entries.len());
	let mut artist_ids = HashSet::<DocumentId>::new();
	let mut cover_ids = Vec::<DocumentId>::new();

	for mapping in &entries {
		let id = mapping.document.header.id.deserialize::<u64>()?;
		let release = Release::document_contents(mapping.document)?;

		for artist in &release.artists {
			artist_ids.insert(DocumentId::from_u64(artist.id));
		}

		if let Some(covers) = &release.cover_ids {
			covers.iter().for_each(|e| cover_ids.push(DocumentId::from_u64(*e)));
		}

		releases.insert(id, release);
	}

	let mut artists = HashMap::<u64, Person>::with_capacity(artist_ids.len());
	let mut covers = HashMap::<u64, DisplayCoverResource>::with_capacity(cover_ids.len());

	for i in Person::get_multiple_async(&artist_ids, database.inner_ref()).await? {
		artists.insert(i.header.id, i.contents);
	}

	for i in Cover::get_multiple_async(&cover_ids, database.inner_ref()).await? {
		covers.insert(
			i.header.id,
			DisplayCoverResource::from_cover(i.contents, &resource_cover_dir),
		);
	}

	debug!("Finished building display release query in {:?}", start.elapsed());

	Ok(DisplayReleases {
		releases,
		artists,
		covers,
	})
}
