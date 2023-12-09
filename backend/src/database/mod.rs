use std::path::Path;

use {
	bonsaidb::{
		core::keyvalue::AsyncKeyValue,
		local::{
			config::{Builder, StorageConfiguration},
			AsyncDatabase as BonsaiDatabase,
		},
	},
	tracing::debug,
};

use crate::{constants::UNKNOWN_PERSON_ID, errors::Result};

use self::models::{person::Person, LocalSchema};

pub mod helpers;
pub mod methods;
pub mod models;
pub mod views;

pub const DB_MAIN_NAME: &str = "main.bonsaidb";
const KEY_IS_FIRST_RUN: &str = "is_first_run";

pub struct Database(pub BonsaiDatabase);

impl Database {
	/// Initialize the database.
	///
	/// This function should run in the context of tauri.
	#[tracing::instrument()]
	pub async fn new(db_path: &Path) -> Result<Self> {
		let db_conf = StorageConfiguration::new(db_path);
		let database = BonsaiDatabase::open::<LocalSchema>(db_conf).await?;
		debug!("Successfully opened the database at {:?}", db_path);

		if database.get_key(KEY_IS_FIRST_RUN).await?.is_none() {
			Self::run_first_time_setup(&database).await?;
		}

		Ok(Self(database))
	}

	#[cfg(test)]
	pub async fn testing() -> Result<Self> {
		let db_dir = std::env::current_dir()
			.unwrap()
			.join("target/testing")
			.join(DB_MAIN_NAME);
		let db_conf = StorageConfiguration::default().path(db_dir).memory_only();
		let database = BonsaiDatabase::open::<LocalSchema>(db_conf).await?;

		Ok(Self(database))
	}

	#[tracing::instrument(skip(database), name = "First time setup")]
	async fn run_first_time_setup(database: &BonsaiDatabase) -> Result<()> {
		let unknown_person = Person::unknown();
		methods::person::insert_with_unique_id(database, unknown_person, UNKNOWN_PERSON_ID).await?;

		database.set_key(KEY_IS_FIRST_RUN, &false).await?;

		Ok(())
	}

	pub fn inner_ref(&self) -> &BonsaiDatabase {
		&self.0
	}
}
