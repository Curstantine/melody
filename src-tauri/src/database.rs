use std::fs;

use polodb_core::Database as PoolDatabase;

use crate::errors::{Error, Result};

pub struct Database {
	inner: PoolDatabase,
}

impl Database {
	/// Initialize the database.
	///
	/// This function should run in the context of tauri.
	pub fn new(app_handle: &tauri::AppHandle) -> Result<Self> {
		let app_data_dir = app_handle
			.path_resolver()
			.app_data_dir()
			.expect("App data dir was not found");

		if let Err(e) = fs::create_dir_all(&app_data_dir) {
			return Err(Error::Io(e, Some("Failed to create app data directory".to_string())));
		}

		let db_path = app_data_dir.join("main.db");
		let database = PoolDatabase::open_file(db_path)?;

		Ok(Self { inner: database })
	}
}
