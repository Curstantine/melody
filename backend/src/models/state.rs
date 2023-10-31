use std::sync::Mutex as BlockingMutex;
use tokio::sync::Mutex;

use crate::{database::Database, errors::Result};

#[derive(Default)]
pub struct AppState {
	pub db: Mutex<Option<Database>>,
	pub initialized: BlockingMutex<bool>,
}

impl AppState {
	pub async fn initialize(&self, app_handle: &tauri::AppHandle) -> Result<()> {
		let db = Database::new(app_handle).await?;
		self.db.lock().await.replace(db);

		let mut init = self.initialized.lock().unwrap();
		*init = true;

		Ok(())
	}
}
