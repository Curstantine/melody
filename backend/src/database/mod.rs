use tokio::fs;

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

		match fs::create_dir_all(&app_data_dir).await {
			Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => {
				return Err(Error::from(e).with_context("Failed to create the app data directory"));
			}
			_ => {}
		}

		let db_file_path = app_data_dir.join(Self::DB_FILE_NAME);
		let db_conf = StorageConfiguration::new(&db_file_path);
		let database = match BonsaiDatabase::open::<LocalSchema>(db_conf).await {
			Ok(db) => db,
			Err(e) => match e {
				bonsaidb::local::Error::Io(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
					let context = format!(
						"A database that doesn't match the expected constraints already exist at {:?}",
						db_file_path
					);
					return Err(Error::from(e).with_context(context));
				}
				_ => {
					let context = format!("Failed to open the database at {:?}", db_file_path);
					return Err(Error::from(e).with_context(context));
				}
			},
		};

		Ok(Self(database))
	}

	#[cfg(test)]
	pub async fn testing() -> Result<Self> {
		let db_dir = std::env::current_dir().unwrap().join("target/testing");
		let db_conf = StorageConfiguration::default()
			.path(db_dir.join(Self::DB_FILE_NAME))
			.memory_only();
		let database = BonsaiDatabase::open::<LocalSchema>(db_conf).await?;

		Ok(Self(database))
	}
}
