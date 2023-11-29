use std::sync::Mutex as BlockingMutex;
use tokio::sync::Mutex;

use crate::{database::Database, errors::Result};

use super::directories::Directories;

#[derive(Default)]
pub struct AppState {
	pub db: Mutex<Option<Database>>,
	pub directories: Mutex<Option<Directories>>,
	pub initialized: BlockingMutex<bool>,
}

impl AppState {
	pub async fn initialize(&self, app_handle: &tauri::AppHandle) -> Result<()> {
		let directories = Directories::new(app_handle).await?;
		let db = Database::new(&directories.database_dir).await?;

		self.directories.lock().await.replace(directories);
		self.db.lock().await.replace(db);

		let mut init = self.initialized.lock().unwrap();
		*init = true;

		Ok(())
	}
}
