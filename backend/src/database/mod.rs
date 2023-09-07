use std::fs;

use bonsaidb::local::{
	config::{Builder, StorageConfiguration},
	AsyncDatabase as BonsaiDatabase,
};

use crate::errors::{Error, Result};

use self::models::LocalSchema;

pub mod models;
pub mod views;

pub struct Database(pub BonsaiDatabase);

impl Database {
	const DB_FILE_NAME: &str = "main.db";

	/// Initialize the database.
	///
	/// This function should run in the context of tauri.
	pub async fn new(app_handle: &tauri::AppHandle) -> Result<Self> {
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

		let db_conf = StorageConfiguration::new(app_data_dir.join(Self::DB_FILE_NAME));
		let database = BonsaiDatabase::open::<LocalSchema>(db_conf).await?;

		Ok(Self(database))
	}
}
