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

use crate::{
	constants::UNKNOWN_PERSON_ID,
	database::{
		constants::KEY_IS_FIRST_RUN,
		models::{person::Person, LocalSchema},
	},
	errors::Result,
};

pub mod constants;
pub mod helpers;
pub mod methods;
pub mod models;
pub mod views;

pub struct Database(pub BonsaiDatabase);

impl Database {
	/// Initialize the database.
	///
	/// This function should run in the context of tauri.
	#[tracing::instrument]
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
		use crate::database::constants::DB_MAIN_NAME;

		let db_dir = std::env::current_dir()
			.unwrap()
			.join("target/testing")
			.join(DB_MAIN_NAME);
		let db_conf = StorageConfiguration::default().path(db_dir).memory_only();
		let database = BonsaiDatabase::open::<LocalSchema>(db_conf).await?;

		Ok(Self(database))
	}

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
