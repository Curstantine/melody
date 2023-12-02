use std::{path::PathBuf, sync::mpsc, thread};

use {
	bonsaidb::core::schema::{SerializedCollection, SerializedView},
	tokio::time::Instant,
	tracing::{debug, info},
};

use crate::{
	database::{
		helpers::handle_temp_track_meta, methods, models::library::Library as LibraryModel,
		views::library::LibraryByName,
	},
	errors::{extra::CopyableSerializableError, Result},
	models::{
		state::AppState,
		tauri::{
			library::{LibraryActionData, LibraryActionPayload, LibraryEntity, LibraryEvent},
			WindowEventManager,
		},
		temp::{TempTrackMeta, TempTrackResource},
	},
	utils::{fs::walkdir_sync, matchers, symphonia::read_track_meta},
};

#[tauri::command]
#[tracing::instrument(skip(app_state))]
pub async fn get_libraries(app_state: tauri::State<'_, AppState>) -> Result<Vec<LibraryEntity>> {
	let db_lock = app_state.db.lock().await;
	let database = db_lock.as_ref().unwrap();
	let database = &database.0;

	let entries = LibraryByName::entries_async(database).query_with_docs().await?;
	let mut names = Vec::with_capacity(entries.len());

	for mapping in &entries {
		let id = mapping.document.header.id.deserialize::<u64>()?;
		let content = LibraryModel::document_contents(mapping.document)?;
		names.push(LibraryEntity::new(id, content));
	}

	Ok(names)
}

#[tauri::command]
#[tracing::instrument(skip_all, err(Debug))]
pub async fn create_library(
	name: String,
	scan_locations: Vec<String>,
	window: tauri::Window,
	app_state: tauri::State<'_, AppState>,
) -> Result<LibraryEntity> {
	let start = Instant::now();

	let db_lock = app_state.db.lock().await;
	let database = db_lock.as_ref().unwrap();
	let database = &database.0;

	let dir_lock = app_state.directories.lock().await;
	let directories = dir_lock.as_ref().unwrap();

	let locs = scan_locations.clone();
	let library = methods::library::insert_unique(database, LibraryModel::new(name, locs)).await?;

	enum ChannelData {
		Error(CopyableSerializableError, PathBuf),
		Reading(LibraryActionData),
		Indexing(LibraryActionData, Box<TempTrackMeta>, TempTrackResource),
	}

	let (tx, rx) = mpsc::channel::<ChannelData>();

	let handle = thread::spawn::<_, Result<()>>(move || {
		let scan_location = scan_locations.into_iter().map(PathBuf::from).collect::<Vec<_>>();

		for scan_location in scan_location {
			let paths = walkdir_sync(&scan_location, matchers::path::audio)?;
			let total = paths.len() as u64;

			for (i, path) in paths.into_iter().enumerate() {
				let current = i as u64 + 1;

				let data = LibraryActionData::new(total, current, path.clone());
				tx.send(ChannelData::Reading(data)).unwrap();

				match read_track_meta(&path) {
					Ok((meta, resources)) => {
						let data = LibraryActionData::new(total, current, path);
						tx.send(ChannelData::Indexing(data, Box::new(meta), resources)).unwrap();
					}
					Err(e) => tx.send(ChannelData::Error(e.into(), path)).unwrap(),
				}
			}
		}

		Ok(())
	});

	let em = WindowEventManager(LibraryEvent::Scan);
	for message in rx {
		match message {
			ChannelData::Reading(payload) => {
				debug!("[{}/{}] Reading: {:#?}", payload.current, payload.total, payload.path);
				em.emit(&window, LibraryActionPayload::reading(payload))?;
			}
			ChannelData::Indexing(payload, meta, resources) => {
				debug!("[{}/{}] Indexing: {:#?}", payload.current, payload.total, payload.path);
				em.emit(&window, LibraryActionPayload::indexing(payload))?;

				handle_temp_track_meta(database, &directories.resource_cover_dir, *meta, resources).await?;
			}
			ChannelData::Error(error, path) => {
				debug!("Error encountered while reading/indexing: {:#?}", path);
				em.emit(&window, LibraryActionPayload::error(error, path))?;
			}
		};
	}

	handle.join().unwrap()?;
	info!("Finished building library in {:?}", start.elapsed());

	Ok(LibraryEntity::new(library.header.id, library.contents))
}
