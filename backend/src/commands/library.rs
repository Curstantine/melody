use std::{
	path::PathBuf,
	sync::{mpsc, Arc},
	thread,
};

use {
	bonsaidb::core::schema::{SerializedCollection, SerializedView},
	tauri::State,
	tokio::{task::JoinSet, time::Instant},
	tracing::{debug, error, info},
};

use crate::{
	database::{
		helpers::handle_temp_track_meta, methods, models::library::Library as LibraryModel,
		views::library::LibraryByName,
	},
	errors::{Error, Result},
	models::{
		state::{DatabaseState, DirectoryState},
		tauri::library::{LibraryEntity, LibraryEventData, LibraryEventManager, LibraryEventPayload, LibraryEventType},
		temp::{TempTrackMeta, TempTrackResource},
	},
	utils::{audio::symphonia::read_track_meta, fs::walkdir_sync, matchers},
};

#[tauri::command]
#[tracing::instrument(skip(db_state))]
pub async fn get_libraries(db_state: State<'_, DatabaseState>) -> Result<Vec<LibraryEntity>> {
	let db_guard = db_state.get().await;
	let database = db_guard.as_ref().unwrap();

	let entries = LibraryByName::entries_async(database.inner_ref())
		.query_with_docs()
		.await?;

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
	dir_state: tauri::State<'_, DirectoryState>,
	db_state: tauri::State<'_, DatabaseState>,
) -> Result<LibraryEntity> {
	let start = Instant::now();

	let locs = scan_locations.clone();
	let library = {
		let db_lock = db_state.get().await;
		let database = db_lock.as_ref().unwrap();
		methods::library::insert_unique(database.inner_ref(), LibraryModel::new(name, locs)).await?
	};

	enum ChannelData {
		Error(Error, PathBuf),
		Reading(LibraryEventData),
		Indexing(LibraryEventData, Box<TempTrackMeta>, TempTrackResource),
	}

	let (tx, rx) = mpsc::channel::<ChannelData>();
	let decode_handle = thread::Builder::new()
		.name("melody_decode".to_string())
		.spawn::<_, Result<()>>(move || {
			let scan_location = scan_locations.into_iter().map(PathBuf::from).collect::<Vec<_>>();

			for scan_location in scan_location {
				let paths = walkdir_sync(&scan_location, matchers::path::audio)?;
				let total = paths.len() as u64;

				for (i, path) in paths.into_iter().enumerate() {
					let current = i as u64 + 1;

					let data = LibraryEventData::new(total, current, path.clone());
					tx.send(ChannelData::Reading(data)).unwrap();

					match read_track_meta(&path) {
						Ok((meta, resources)) => {
							let data = LibraryEventData::new(total, current, path);
							tx.send(ChannelData::Indexing(data, Box::new(meta), resources)).unwrap();
						}
						Err(e) => tx.send(ChannelData::Error(e, path)).unwrap(),
					}
				}
			}

			Ok(())
		})
		.unwrap();

	let db_arc = Arc::clone(&db_state.0);
	let cover_dir_arc: Arc<PathBuf> = {
		let dir_guard = dir_state.get();
		let directories = dir_guard.as_ref().unwrap();
		Arc::new(directories.resource_cover_dir.clone())
	};

	let em = LibraryEventManager::new(LibraryEventType::Scan);
	let mut task_set = JoinSet::<Result<()>>::new();

	for message in rx.into_iter() {
		match message {
			ChannelData::Reading(payload) => {
				debug!("[{}/{}] Reading: {:#?}", payload.current, payload.total, payload.path);
				em.emit(&window, LibraryEventPayload::reading(payload))?;
			}
			ChannelData::Indexing(payload, meta, resources) => {
				debug!("[{}/{}] Indexing: {:#?}", payload.current, payload.total, payload.path);
				em.emit(&window, LibraryEventPayload::indexing(payload))?;

				let db_ar = Arc::clone(&db_arc);
				let dir_ar = Arc::clone(&cover_dir_arc);

				task_set.spawn(async move {
					let db_guard = db_ar.lock().await;
					let database = db_guard.as_ref().unwrap();
					handle_temp_track_meta(database.inner_ref(), &dir_ar, *meta, resources).await?;

					Ok(())
				});
			}
			ChannelData::Error(error, path) => {
				error!("Error encountered while reading/indexing: {:#?}", path);
				em.emit(&window, LibraryEventPayload::error(error, path))?;
			}
		};
	}

	decode_handle.join().unwrap()?;
	while let Some(res) = task_set.join_next().await {
		res.unwrap()?;
	}

	info!("Finished building library in {:?}", start.elapsed());

	Ok(LibraryEntity::new(library.header.id, library.contents))
}
