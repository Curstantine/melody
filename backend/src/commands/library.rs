use std::path::PathBuf;

use bonsaidb::core::schema::{SerializedCollection, SerializedView};
use futures::TryStreamExt;
use tokio::{fs, task::JoinSet, time::Instant};
use tracing::{debug, info};

use crate::{
	constants::SUPPORTED_AUDIO_EXTENSIONS,
	database::{
		models::{library::Library as LibraryModel, release::Release},
		views::{
			label::LabelByName,
			library::LibraryByName,
			person::{PersonByNameAndSort, PersonByNameAndSortData},
			tag::TagByNameAndType,
		},
	},
	errors::{Error, Result},
	models::{
		state::AppState,
		tauri::{
			library::{LibraryActionType, LibraryEvent, LibraryGenericActionPayload},
			WindowEvent,
		},
		temp::TempTrackMeta,
	},
	utils::{self, symphonia::read_track_meta},
};

#[tauri::command]
#[tracing::instrument(skip(app_state))]
pub async fn get_library_names(app_state: tauri::State<'_, AppState>) -> Result<Vec<String>> {
	let db_lock = app_state.db.lock().await;
	let database = db_lock.as_ref().unwrap();
	let database = &database.0;

	let libraries = LibraryByName::entries_async(database).query_with_docs().await?;
	let names = libraries.into_iter().map(|x| x.key.clone()).collect::<Vec<_>>();

	Ok(names)
}

#[tauri::command]
#[tracing::instrument(skip(window, app_state))]
pub async fn create_library(
	name: String,
	scan_locations: Vec<String>,
	window: tauri::Window,
	app_state: tauri::State<'_, AppState>,
) -> Result<()> {
	let start = Instant::now();

	let db_lock = app_state.db.lock().await;
	let database = db_lock.as_ref().unwrap();
	let database = &database.0;

	let docs = LibraryByName::entries_async(database).with_key(&name).query().await?;
	if !docs.is_empty() {
		let message = format!("A library with the name {} already exists", name);
		return Err(Error::descriptive(message));
	}

	let hi = LibraryModel {
		name,
		scan_locations: scan_locations.clone(),
	}
	.push_into_async(database)
	.await?;

	debug!("Created library: {:#?}", hi);

	for scan_location in scan_locations {
		debug!("Scanning {}", scan_location);

		// We could iterate on the stream, but I feel like it wouldn't make a big difference for the complexity of the implementation.
		let paths = utils::fs::walkdir(scan_location)
			.try_filter(|p| {
				let path = p.path();
				let is_readable = matches!(path.extension(), Some(extension) if SUPPORTED_AUDIO_EXTENSIONS.contains(&extension.to_str().unwrap()));

				std::future::ready(is_readable)
			})
			.try_collect::<Vec<_>>()
			.await?;

		let path_len = paths.len();
		let mut probe_tasks = JoinSet::<Result<(PathBuf, TempTrackMeta)>>::new();

		for (i, entry) in paths.into_iter().enumerate() {
			let path = entry.path();
			let extension = match path.extension() {
				Some(extension) => extension.to_str().unwrap().to_string(),
				_ => continue,
			};

			WindowEvent::new(
				LibraryEvent::Scan,
				LibraryGenericActionPayload {
					action_type: LibraryActionType::Reading,
					total: path_len as u32,
					current: i as u32 + 1,
					path: entry.path(),
				},
			)
			.emit(&window)?;
			debug!("Reading [{}/{}], currently reading:\n{:#?}", i + 1, path_len, path);

			let src = fs::File::open(&path).await?.into_std().await;
			probe_tasks.spawn_blocking(move || {
				let meta = read_track_meta(Box::new(src), Some(&extension))?;
				Ok((path, meta))
			});
		}

		let mut idx: u32 = 0;

		while let Some(x) = probe_tasks.join_next().await.transpose()? {
			let (path, meta) = x?;
			idx += 1;

			debug!("Probed [{}/{}], currently indexing:\n{:#?}", idx, path_len, path);
			WindowEvent::new(
				LibraryEvent::Scan,
				LibraryGenericActionPayload {
					action_type: LibraryActionType::Indexing,
					total: path_len as u32,
					current: idx,
					path,
				},
			)
			.emit(&window)?;

			// NOTE:
			// We might not need to spawn tasks here,
			// since we could come across race conditions on which duplicated entry to put into the db, lol.
			let mut artist_ids = Vec::<u64>::with_capacity(meta.artists.as_ref().map(|x| x.len()).unwrap_or(0));
			let mut composer_ids = Vec::<u64>::with_capacity(meta.composers.as_ref().map(|x| x.len()).unwrap_or(0));
			let mut producer_ids = Vec::<u64>::with_capacity(meta.producers.as_ref().map(|x| x.len()).unwrap_or(0));

			let mut label_ids = Vec::<u64>::with_capacity(meta.labels.as_ref().map(|x| x.len()).unwrap_or(0));
			let mut genre_ids = Vec::<u64>::with_capacity(meta.genres.as_ref().map(|x| x.len()).unwrap_or(0));
			let mut tag_ids = Vec::<u64>::with_capacity(meta.tags.as_ref().map(|x| x.len()).unwrap_or(0));

			if let Some(temp_artists) = meta.artists {
				for temp_artist in temp_artists {
					let id = PersonByNameAndSort::put_or_get(database, temp_artist).await?;
					artist_ids.push(id);
				}
			}

			if let Some(temp_composers) = meta.composers {
				for temp_composer in temp_composers {
					let id = PersonByNameAndSort::put_or_get(database, temp_composer).await?;
					composer_ids.push(id);
				}
			}

			if let Some(temp_producers) = meta.producers {
				for temp_producer in temp_producers {
					let id = PersonByNameAndSort::put_or_get(database, temp_producer).await?;
					producer_ids.push(id);
				}
			}

			if let Some(temp_labels) = meta.labels {
				for temp_label in temp_labels {
					let id = LabelByName::put_or_get(database, temp_label).await?;
					label_ids.push(id);
				}
			}

			if let Some(temp_genres) = meta.genres {
				for temp_genre in temp_genres {
					let id = TagByNameAndType::put_or_get(database, temp_genre).await?;
					genre_ids.push(id);
				}
			}

			if let Some(temp_tags) = meta.tags {
				for temp_tag in temp_tags {
					let id = TagByNameAndType::put_or_get(database, temp_tag).await?;
					tag_ids.push(id);
				}
			}

			// if let Some(temp_release) = meta.release {}
		}
	}

	let elapsed = start.elapsed();
	info!("Finished building library in {:?}", elapsed);

	Ok(())
}
