use std::path::PathBuf;

use {
	bonsaidb::core::schema::{SerializedCollection, SerializedView},
	tauri::State,
	tokio::time::Instant,
	tracing::{debug, error, info},
};

use crate::{
	database::{
		helpers::handle_temp_track_meta, methods, models::library::Library as LibraryModel,
		views::library::LibraryByName,
	},
	errors::Result,
	ffmpeg::meta::read_track_meta,
	models::{
		state::{DatabaseState, DirectoryState},
		tauri::library::{LibraryEntity, LibraryEventData, LibraryEventManager, LibraryEventPayload, LibraryEventType},
	},
	utils::{fs::walkdir_sync, matchers},
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

	let cover_dir: PathBuf = {
		let dir_guard = dir_state.get();
		let directories = dir_guard.as_ref().unwrap();
		directories.resource_cover_dir.clone()
	};

	let em = LibraryEventManager::new(LibraryEventType::Scan);
	let locations = scan_locations.into_iter().map(PathBuf::from).collect::<Vec<_>>();

	for location in locations {
		info!("Scanning: {location:?}");
		em.emit(&window, LibraryEventPayload::scanning(location.clone()))?;

		let (paths, total) = {
			let x = location.clone();
			let paths = tokio::task::spawn_blocking(move || walkdir_sync(&x, matchers::path::audio)).await??;
			let len = paths.len() as u64;
			(paths, len)
		};

		info!("Scan location '{location:?}' found with {total} files");

		for (i, path) in paths.into_iter().enumerate() {
			let current = i as u64 + 1;

			debug!("[{current}/{total}] Reading: {path:#?}");
			let payload = LibraryEventData::new(total, current, path.clone());
			em.emit(&window, LibraryEventPayload::reading(payload))?;

			let meta = {
				let x = path.clone();
				tokio::task::spawn_blocking(move || read_track_meta(&x)).await?
			};

			match meta {
				Ok((meta, resources)) => {
					let db_lock = db_state.get().await;
					let database = db_lock.as_ref().unwrap();

					debug!("[{current}/{total}] Indexing: {path:#?}");
					let payload = LibraryEventData::new(total, current, path);
					em.emit(&window, LibraryEventPayload::indexing(payload))?;

					handle_temp_track_meta(database.inner_ref(), &cover_dir, meta, resources).await?;
				}
				Err(e) => {
					error!("Error encountered while reading/indexing: {path:#?}\n{e:#?}");
					em.emit(&window, LibraryEventPayload::error(e, path))?
				}
			}
		}
	}

	info!("Finished building library in {:?}", start.elapsed());

	Ok(LibraryEntity::new(library.header.id, library.contents))
}
