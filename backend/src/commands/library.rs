use std::path::PathBuf;

use bonsaidb::core::schema::{SerializedCollection, SerializedView};
use futures::TryStreamExt;
use tokio::{fs, task::JoinSet};
use tracing::{debug, info};

use crate::{
	constants::SUPPORTED_AUDIO_EXTENSIONS,
	database::{models::library::Library as LibraryModel, views::library::LibraryByName},
	errors::{Error, Result},
	models::{
		state::AppState,
		tauri::{
			library::{LibraryActionType, LibraryEvent, LibraryGenericActionPayload},
			WindowEvent,
		},
	},
	utils::{
		self,
		symphonia::{read_track_meta, TempMeta},
	},
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
		let mut sync_threads = JoinSet::<Result<(PathBuf, TempMeta)>>::new();

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
			sync_threads.spawn_blocking(move || {
				let meta = read_track_meta(Box::new(src), Some(&extension))?;
				Ok((path, meta))
			});
		}

		let mut idx: u32 = 0;
		while let Some(x) = sync_threads.join_next().await.transpose()? {
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

			if let Some(release) = meta.release {}
		}
	}

	info!("Finished building library.");

	Ok(())
}
