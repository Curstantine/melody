use std::{path::PathBuf, sync::mpsc, thread};

use {
	tokio::time::Instant,
	tracing::{debug, error, info},
};

use crate::{
	database::{helpers::handle_temp_track_meta, methods},
	errors::{Error, Result},
	ffmpeg::meta::read_track_meta,
	models::{
		state::{DatabaseState, DirectoryState},
		tauri::library::{LibraryEventData, LibraryEventManager, LibraryEventPayload, LibraryEventType},
		temp::{TempTrackMeta, TempTrackResource},
	},
	utils::{fs::walkdir_sync, matchers},
};

#[tauri::command]
#[tracing::instrument(skip(db_state), err(Debug))]
pub async fn get_scan_locations(db_state: tauri::State<'_, DatabaseState>) -> Result<Option<Vec<String>>> {
	let db_lock = db_state.get().await;
	let database = db_lock.as_ref().unwrap();

	methods::library::get_scan_locations(database.inner_ref()).await
}

#[tauri::command]
#[tracing::instrument(skip(window, dir_state, db_state), err(Debug))]
pub async fn initialize_library(
	scan_locations: Vec<String>,
	window: tauri::Window,
	dir_state: tauri::State<'_, DirectoryState>,
	db_state: tauri::State<'_, DatabaseState>,
) -> Result<()> {
	let start = Instant::now();

	{
		let db_lock = db_state.get().await;
		let database = db_lock.as_ref().unwrap();
		methods::library::set_scan_locations(database.inner_ref(), &scan_locations).await?;
	};

	#[derive(Debug)]
	enum ChannelData {
		Scanning(PathBuf),
		Finished((u64, u64), PathBuf, Box<TempTrackMeta>, TempTrackResource),
		Err(Error, PathBuf),
	}

	let (tx, rx) = mpsc::channel::<ChannelData>();
	let probe_handle = thread::Builder::new()
		.name("melody_probe".to_string())
		.spawn::<_, Result<()>>(move || {
			for location in scan_locations.iter().map(PathBuf::from) {
				tx.send(ChannelData::Scanning(location.clone()))?;

				let paths = walkdir_sync(&location, matchers::path::audio)?;
				let total = paths.len() as u64;

				for (i, path) in paths.into_iter().enumerate() {
					let data = match read_track_meta(&path) {
						Ok((x, y)) => ChannelData::Finished((i as u64 + 1, total), path.clone(), Box::new(x), y),
						Err(e) => ChannelData::Err(e, path.clone()),
					};

					tx.send(data)?;
				}
			}

			Ok(())
		})
		.unwrap();

	let em = LibraryEventManager::new(LibraryEventType::Scan);
	let cover_dir: PathBuf = {
		let dir_guard = dir_state.get();
		let directories = dir_guard.as_ref().unwrap();
		directories.cover_dir.clone()
	};

	for result in rx.into_iter() {
		match result {
			ChannelData::Scanning(location) => {
				debug!("Scanning location '{location:?}'");
				em.emit(&window, LibraryEventPayload::scanning(location))?;
			}
			ChannelData::Finished((current, total), path, meta, resources) => {
				let db_lock = db_state.get().await;
				let database = db_lock.as_ref().unwrap();

				debug!("[{current}/{total}] Indexing: {path:#?}");
				let payload = LibraryEventData::new(total, current, path);
				em.emit(&window, LibraryEventPayload::indexing(payload))?;

				handle_temp_track_meta(database.inner_ref(), &cover_dir, *meta, resources).await?;
			}
			ChannelData::Err(e, path) => {
				error!("Error encountered while reading/indexing: {path:#?}\n{e:#?}");
				em.emit(&window, LibraryEventPayload::error(e, path))?
			}
		}
	}

	probe_handle.join().unwrap()?;

	info!("Finished building library in {:?}", start.elapsed());

	Ok(())
}
