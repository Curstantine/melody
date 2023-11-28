use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use bonsaidb::core::schema::SerializedView;
use tokio::time::Instant;
use tracing::{debug, info};

use crate::{
	database::{
		helpers::{handle_temp_track_meta, handle_temp_track_resources},
		methods,
		models::library::Library as LibraryModel,
		views::library::LibraryByName,
	},
	errors::{extra::CopyableSerializableError, Error, FromErrorWithContextData, IoErrorType, Result},
	models::{
		state::AppState,
		tauri::{
			library::{LibraryActionData, LibraryActionPayload, LibraryEvent},
			WindowEventManager,
		},
		temp::{TempTrackMeta, TempTrackResource},
	},
	utils::{fs::walkdir_sync, matchers, symphonia::read_track_meta},
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
#[tracing::instrument(skip_all, err(Debug))]
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

	let dir_lock = app_state.directories.lock().await;
	let directories = dir_lock.as_ref().unwrap();

	let library = LibraryModel::new(name.clone(), scan_locations.clone());
	methods::library::insert_unique(database, library).await?;

	enum ChannelData {
		Error(CopyableSerializableError, PathBuf),
		Reading(LibraryActionData),
		Indexing(LibraryActionData, Box<TempTrackMeta>, TempTrackResource),
	}

	let (tx, rx) = mpsc::channel::<ChannelData>();

	let handle = thread::spawn::<_, Result<()>>(move || {
		let scan_location = scan_locations.into_iter().map(PathBuf::from).collect::<Vec<_>>();

		for scan_location in scan_location {
			let paths = walkdir_sync(&scan_location, matchers::path::audio)
				.map_err(|x| Error::from_with_ctx(x, IoErrorType::Path(&scan_location)))?;
			let total = paths.len() as u64;

			for (i, path) in paths.into_iter().enumerate() {
				let current = i as u64 + 1;

				let data = LibraryActionData::reading(total, current, path.clone());
				tx.send(ChannelData::Reading(data)).unwrap();

				match read_track_meta(&path) {
					Ok((meta, resources)) => {
						let data = LibraryActionData::indexing(total, current, path);
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
				em.emit(&window, LibraryActionPayload::Ok(payload))?;
			}
			ChannelData::Indexing(payload, meta, resources) => {
				debug!("[{}/{}] Indexing: {:#?}", payload.current, payload.total, payload.path);
				em.emit(&window, LibraryActionPayload::Ok(payload))?;

				handle_temp_track_resources(database, &directories.resource_cover_dir, resources).await?;
				handle_temp_track_meta(database, *meta).await?;
			}
			ChannelData::Error(error, path) => {
				debug!("Error encountered while reading/indexing: {:#?}", path);
				em.emit(&window, LibraryActionPayload::Error { error, path })?;
			}
		};
	}

	handle.join().unwrap()?;

	let elapsed = start.elapsed();
	info!("Finished building library in {:?}", elapsed);

	Ok(())
}
