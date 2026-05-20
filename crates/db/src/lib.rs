pub mod models;

use std::{
	fs,
	sync::{Arc, Mutex, MutexGuard},
};

use anyhow::{Context, anyhow};
use paths::database_dir;
use rusqlite::Connection;

#[derive(Clone)]
pub struct AppDatabase(Arc<Mutex<Connection>>);

impl gpui::Global for AppDatabase {}

impl AppDatabase {
	pub fn new() -> anyhow::Result<Self> {
		let dir = database_dir();
		let file = dir.join("db.sqlite3");
		fs::create_dir_all(dir).context("Failed to create db directory")?;

		let connection = Connection::open(file).context("Failed to open a connection to the db")?;
		Ok(Self(Arc::new(Mutex::new(connection))))
	}

	pub fn connection(&self) -> anyhow::Result<MutexGuard<'_, Connection>> {
		self.0.lock().map_err(|error| anyhow!("Db mutex was poisoned: {error}"))
	}
}
