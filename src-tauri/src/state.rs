use std::sync::Mutex;

use crate::{database::Database, errors::Result};

#[derive(Default)]
pub struct AppState {
	pub db: Mutex<Option<Database>>,
}

impl AppState {
	pub fn initialize(&self, app_handle: &tauri::AppHandle) -> Result<()> {
		let db = Database::new(app_handle)?;
		self.db.lock().unwrap().replace(db);

		Ok(())
	}
}
