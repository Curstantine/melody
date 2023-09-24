use tokio::sync::Mutex;

use crate::{database::Database, errors::Result};

#[derive(Default)]
pub struct AppState {
	pub db: Mutex<Option<Database>>,
}

impl AppState {
	#[tracing::instrument(skip(self, app_handle))]
	pub async fn initialize(&self, app_handle: &tauri::AppHandle) -> Result<()> {
		let db = Database::new(app_handle).await?;
		self.db.try_lock().unwrap().replace(db);

		Ok(())
	}
}
