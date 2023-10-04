use tokio::fs;

use bonsaidb::{
	core::keyvalue::AsyncKeyValue,
	local::{
		config::{Builder, StorageConfiguration},
		AsyncDatabase as BonsaiDatabase,
	},
};
use tracing::debug;

use crate::{
	constants::UNKNOWN_PERSON_ID,
	errors::{Error, Result},
};

use self::models::{person::Person, LocalSchema};

pub mod models;
pub mod views;

pub struct Database(pub BonsaiDatabase);

impl Database {
	const DB_NAME: &str = "database";

	const KEY_IS_FIRST_RUN: &str = "is_first_run";

	/// Initialize the database.
	///
	/// This function should run in the context of tauri.
	#[tracing::instrument(skip(app_handle))]
	pub async fn new(app_handle: &tauri::AppHandle) -> Result<Self> {
		let app_data_dir = app_handle
			.path_resolver()
			.app_data_dir()
			.expect("App data dir was not found");

		debug!("Found app data dir at {:?}", app_data_dir);

		match fs::create_dir_all(&app_data_dir).await {
			Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => {
				return Err(Error::from(e).with_context("Failed to create the app data directory"));
			}
			_ => {}
		}

		let db_file_path = app_data_dir.join(Self::DB_NAME);
		let db_conf = StorageConfiguration::new(&db_file_path);
		let database = match BonsaiDatabase::open::<LocalSchema>(db_conf).await {
			Ok(db) => db,
			Err(e) => match e {
				bonsaidb::local::Error::Io(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
					let context = format!(
						"A database that doesn't match the expected constraints already exists at {:?}",
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

		debug!("Successfully opened the database at {:?}", db_file_path);

		if database.get_key(Self::KEY_IS_FIRST_RUN).await?.is_none() {
			Self::run_first_time_setup(&database).await?;
		}

		Ok(Self(database))
	}

	#[cfg(test)]
	pub async fn testing() -> Result<Self> {
		let db_dir = std::env::current_dir().unwrap().join("target/testing");
		let db_conf = StorageConfiguration::default()
			.path(db_dir.join(Self::DB_NAME))
			.memory_only();
		let database = BonsaiDatabase::open::<LocalSchema>(db_conf).await?;

		Ok(Self(database))
	}

	#[tracing::instrument(skip(database), name = "First time setup")]
	async fn run_first_time_setup(database: &BonsaiDatabase) -> Result<()> {
		let unknown_person = Person::unknown();
		unknown_person.set_unique_with_id(database, UNKNOWN_PERSON_ID).await?;

		database.set_key(Self::KEY_IS_FIRST_RUN, &false).await?;

		Ok(())
	}
}
