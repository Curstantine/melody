use futures::StreamExt;
use polodb_core::bson::doc;

use crate::{
	errors::{Error, Result},
	models::{
		library::{Library as LibraryModel, LibraryEvent, LibraryScanEventPayload},
		tauri::WindowEvent,
	},
	state::AppState,
	utils,
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
	let col = database.library();

	let matches = col.find_one(doc! { "name": name.clone() })?;
	if matches.is_some() {
		let message = format!("A library with the name {} already exists", name);
		return Err(Error::Descriptive(message));
	}

	let model = LibraryModel { name, scan_locations };
	col.insert_one(&model)?;

	for scan_location in &model.scan_locations {
		// We could iterate on the stream, but I feel like it wouldn't make a big difference for the complexity of the implementation.
		let paths = utils::fs::walkdir(scan_location).collect::<Vec<_>>().await;
		let path_len = paths.len();

		for (i, entry) in paths.into_iter().enumerate() {
			let entry = entry?;
			let event = WindowEvent::new(
				LibraryEvent::Scan,
				LibraryScanEventPayload {
					total: path_len as u32,
					current: i as u32 + 1,
					path: entry.path(),
				},
			);

			event.emit(&window)?;
		}
	}

	Ok(())
}
