use std::{
	path::Path,
	sync::{Arc, Mutex as BlockingMutex, MutexGuard as BlockingMutexGuard},
};

use {
	tauri::PathResolver,
	tokio::sync::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard},
	tracing::debug,
};

use crate::{
	database::Database,
	errors::Result,
	models::{configuration::Configuration, directories::Directories},
};

#[derive(Default)]
pub struct AppState {
	pub initialized: BlockingMutex<bool>,
}

#[derive(Default)]
pub struct DirectoryState(pub BlockingMutex<Option<Directories>>);

#[derive(Default)]
pub struct ConfigurationState(pub Arc<BlockingMutex<Option<Configuration>>>);

#[derive(Default)]
pub struct DatabaseState(pub Arc<AsyncMutex<Option<Database>>>);

impl AppState {
	pub fn initialize(&self) -> std::result::Result<(), ()> {
		if *self.initialized.lock().unwrap() {
			debug!("AppState::initialize hook reran while the app is initialized. Ignoring...");
			return Err(());
		}

		*self.initialized.lock().unwrap() = true;
		Ok(())
	}
}

impl DirectoryState {
	pub async fn initialize(&self, path_resolver: PathResolver) -> Result<()> {
		let db = Directories::initialize(path_resolver).await?;
		self.get().replace(db);

		Ok(())
	}

	#[inline(always)]
	pub fn get(&self) -> BlockingMutexGuard<'_, Option<Directories>> {
		self.0.lock().unwrap()
	}
}

impl ConfigurationState {
	#[inline(always)]
	pub fn get(&self) -> BlockingMutexGuard<'_, Option<Configuration>> {
		self.0.lock().unwrap()
	}
}

impl DatabaseState {
	pub async fn initialize(&self, database_dir: &Path) -> Result<()> {
		let db = Database::new(database_dir).await?;
		self.0.lock().await.replace(db);

		Ok(())
	}

	#[inline(always)]
	pub async fn get(&self) -> AsyncMutexGuard<'_, Option<Database>> {
		self.0.lock().await
	}
}
