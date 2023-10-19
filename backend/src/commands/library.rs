use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use bonsaidb::core::schema::SerializedView;
use tokio::time::Instant;
use tracing::{debug, info};

use crate::{
	database::{
		helpers::handle_temp_track_meta, methods, models::library::Library as LibraryModel,
		views::library::LibraryByName,
	},
	errors::{extra::CopyableSerializableError, Error, FromErrorWithContextData, IoErrorType, Result},
	models::{
		state::AppState,
		tauri::{
			library::{LibraryActionData, LibraryActionPayload, LibraryEvent},
			WindowEvent,
		},
		temp::TempTrackMeta,
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
#[tracing::instrument(skip(window, app_state), err(Debug))]
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

	let library = LibraryModel::new(name.clone(), scan_locations.clone());
	methods::library::insert_unique(database, library).await?;

	enum ChannelData {
		Error(CopyableSerializableError, PathBuf),
		Reading(LibraryActionData),
		Indexing(LibraryActionData, Box<TempTrackMeta>),
	}

	let (tx, rx) = mpsc::channel::<ChannelData>();
	let scan_location = scan_locations.into_iter().map(PathBuf::from).collect::<Vec<_>>();

	let handle = thread::spawn::<_, Result<()>>(move || {
		for scan_location in scan_location {
			let paths = walkdir_sync(&scan_location, matchers::path::audio)
				.map_err(|x| Error::from_with_ctx(x, IoErrorType::Path(&scan_location)))?;
			let total = paths.len() as u64;

			for (i, path) in paths.into_iter().enumerate() {
				let current = i as u64 + 1;

				let event = LibraryActionData::reading(total, current, path.clone());
				tx.send(ChannelData::Reading(event)).unwrap();

				match read_track_meta(&path) {
					Ok(meta) => {
						let event = LibraryActionData::indexing(total, current, path);
						tx.send(ChannelData::Indexing(event, Box::new(meta))).unwrap();
					}
					Err(e) => tx.send(ChannelData::Error(e.into(), path)).unwrap(),
				}
			}
		}

		Ok(())
	});

	for message in rx {
		match message {
			ChannelData::Reading(payload) => {
				debug!("[{}/{}] Reading: {:#?}", payload.current, payload.total, payload.path);
				WindowEvent::new(LibraryEvent::Scan, LibraryActionPayload::Ok(payload)).emit(&window)?;
			}
			ChannelData::Indexing(payload, meta) => {
				debug!("[{}/{}] Indexing: {:#?}", payload.current, payload.total, payload.path);
				WindowEvent::new(LibraryEvent::Scan, LibraryActionPayload::Ok(payload)).emit(&window)?;

				handle_temp_track_meta(database, *meta).await?;
			}
			ChannelData::Error(error, path) => {
				debug!("Error encountered while reading/indexing: {:#?}", path);
				WindowEvent::new(LibraryEvent::Scan, LibraryActionPayload::Error { error, path }).emit(&window)?;
			}
		};
	}

	handle.join().unwrap()?;

	let elapsed = start.elapsed();
	info!("Finished building library in {:?}", elapsed);

	Ok(())
}
