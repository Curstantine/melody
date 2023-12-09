use std::{
	path::Path,
	sync::{Arc, Mutex as BlockingMutex},
};

use {
	tauri::PathResolver,
	tokio::sync::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard},
};

use crate::{database::Database, errors::Result};

use super::directories::Directories;

#[derive(Default)]
pub struct AppState {
	pub initialized: BlockingMutex<bool>,
}

#[derive(Default)]
pub struct DirectoryState(pub AsyncMutex<Option<Directories>>);

#[derive(Default)]
pub struct DatabaseState(pub Arc<AsyncMutex<Option<Database>>>);

impl AppState {
	pub fn initialize(&self) {
		*self.initialized.lock().unwrap() = true;
	}
}

impl DirectoryState {
	pub async fn initialize(&self, path_resolver: PathResolver) -> Result<()> {
		let db = Directories::new(path_resolver).await?;
		self.get().await.replace(db);

		Ok(())
	}

	#[inline]
	pub async fn get(&self) -> AsyncMutexGuard<'_, Option<Directories>> {
		self.0.lock().await
	}
}

impl DatabaseState {
	pub async fn initialize(&self, database_dir: &Path) -> Result<()> {
		let db = Database::new(database_dir).await?;
		self.0.lock().await.replace(db);

		Ok(())
	}

	#[inline]
	pub async fn get(&self) -> AsyncMutexGuard<'_, Option<Database>> {
		self.0.lock().await
	}
}
