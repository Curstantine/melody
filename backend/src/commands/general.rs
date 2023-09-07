use tauri::Manager;

use crate::{errors::Result, models::state::AppState};

#[tauri::command]
pub async fn setup(app: tauri::AppHandle) -> Result<()> {
	let state = app.state::<AppState>();
	state.initialize(&app).await?;

	Ok(())
}
