use polodb_core::bson::doc;

use crate::{
	errors::{Error, Result},
	models::library::Library as LibraryModel,
	state::AppState,
};

#[tauri::command(async)]
pub fn create(app_state: tauri::State<'_, AppState>, name: String, scan_locations: Vec<String>) -> Result<()> {
	let db_lock = app_state.db.lock().unwrap();
	let database = db_lock.as_ref().unwrap();
	let col = database.library();

	let matches = col.find_one(doc! { "name": name.clone() })?;
	if matches.is_some() {
		let message = format!("A library with the name {} already exists", name);
		return Err(Error::Descriptive(message));
	}

	col.insert_one(LibraryModel { name, scan_locations })?;

	Ok(())
}
