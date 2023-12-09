use std::collections::{HashMap, HashSet};

use {
	bonsaidb::core::{
		document::DocumentId,
		schema::{SerializedCollection, SerializedView},
	},
	tauri::State,
	tokio::time::Instant,
	tracing::info,
};

use crate::{
	database::{
		models::{person::Person, release::Release, resource::Resource},
		views::release::ReleaseByNameAndArtist,
	},
	errors::Result,
	models::{
		state::{DatabaseState, DirectoryState},
		tauri::{
			release::{DisplayReleases, ReleaseEntity},
			resource::DisplayImageResource,
		},
	},
};

#[tauri::command]
#[tracing::instrument(skip(db_state), err(Debug))]
pub async fn get_releases(library_id: u64, db_state: State<'_, DatabaseState>) -> Result<Vec<ReleaseEntity>> {
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
	library_id: u64,
	dir_state: State<'_, DirectoryState>,
	db_state: State<'_, DatabaseState>,
) -> Result<DisplayReleases> {
	let start = Instant::now();

	let dir_guard = dir_state.get().await;
	let db_guard = db_state.get().await;

	let directories = dir_guard.as_ref().unwrap();
	let database = db_guard.as_ref().unwrap();

	let entries = ReleaseByNameAndArtist::entries_async(database.inner_ref())
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
	let mut covers = HashMap::<u64, DisplayImageResource>::with_capacity(cover_ids.len());

	for i in Person::get_multiple_async(&artist_ids, database.inner_ref()).await? {
		artists.insert(i.header.id, i.contents);
	}

	for i in Resource::get_multiple_async(&cover_ids, database.inner_ref()).await? {
		covers.insert(
			i.header.id,
			DisplayImageResource::from_resource(&directories.resource_cover_dir, i.contents),
		);
	}

	info!("Finished building display release query in {:?}", start.elapsed());

	Ok(DisplayReleases {
		releases,
		artists,
		covers,
	})
}
