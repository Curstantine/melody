use std::path::PathBuf;

use bonsaidb::core::{connection::AsyncConnection, schema::SerializedCollection};
use futures::StreamExt;
use tokio::{fs, task::JoinSet};

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
pub async fn create_library(
	name: String,
	scan_locations: Vec<String>,
	window: tauri::Window,
	app_state: tauri::State<'_, AppState>,
) -> Result<()> {
	let db_lock = app_state.db.lock().await;
	let database = db_lock.as_ref().unwrap();
	let database = &database.0;

	let docs = database
		.view::<LibraryByName>()
		.with_key(name.clone())
		.query_with_docs()
		.await?;
	if !docs.is_empty() {
		let message = format!("A library with the name {} already exists", name);
		return Err(Error::descriptive(message));
	}

	LibraryModel {
		name,
		scan_locations: scan_locations.clone(),
	}
	.push_into_async(database)
	.await?;

	for scan_location in scan_locations {
		// We could iterate on the stream, but I feel like it wouldn't make a big difference for the complexity of the implementation.
		let paths = utils::fs::walkdir(scan_location).collect::<Vec<_>>().await;
		let path_len = paths.len();
		let mut sync_threads = JoinSet::<Result<(PathBuf, TempMeta)>>::new();

		for (i, entry) in paths.into_iter().enumerate() {
			let entry = entry?;
			let path = entry.path();
			let extension = match path.extension() {
				Some(extension) if SUPPORTED_AUDIO_EXTENSIONS.contains(&extension.to_str().unwrap()) => {
					extension.to_str().unwrap().to_string()
				}
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

			let src = fs::File::open(&path).await?.into_std().await;
			sync_threads.spawn_blocking(move || {
				let meta = read_track_meta(Box::new(src), Some(&extension))?;
				Ok((path, meta))
			});
		}

		let mut idx: u32 = 0;
		while let Some(x) = sync_threads.join_next().await.transpose()? {
			let (path, _) = x?;
			idx += 1;

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
		}
	}

	Ok(())
}
