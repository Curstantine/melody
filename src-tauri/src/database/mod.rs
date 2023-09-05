use std::fs;

use polodb_core::{Collection as PoloCollection, Database as PoloDatabase};

use crate::{
	errors::{Error, Result},
	models::library::Library as LibraryModel,
};

pub struct Database {
	inner: PoloDatabase,
}

impl Database {
	const DB_FILE_NAME: &str = "main.db";
	const COL_LIBRARY: &str = "library";

	/// Initialize the database.
	///
	/// This function should run in the context of tauri.
	pub fn new(app_handle: &tauri::AppHandle) -> Result<Self> {
		let app_data_dir = app_handle
			.path_resolver()
			.app_data_dir()
			.expect("App data dir was not found");

		match fs::create_dir_all(&app_data_dir) {
			Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => {
				return Err(Error::Io(e, Some("Failed to create app data directory".to_string())))
			}
			_ => {}
		}

		let db_path = app_data_dir.join(Self::DB_FILE_NAME);
		let database = PoloDatabase::open_file(db_path)?;

		let collections = database.list_collection_names()?;
		if collections.is_empty() {
			database.create_collection("library")?;
			database.create_collection("settings")?;
		}

		Ok(Self { inner: database })
	}

	pub fn library(&self) -> PoloCollection<LibraryModel> {
		self.inner.collection::<LibraryModel>(Self::COL_LIBRARY)
	}
}
