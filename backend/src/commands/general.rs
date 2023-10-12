use tauri::Manager;
use tracing::info;

use crate::{errors::Result, models::state::AppState};

#[tauri::command]
#[tracing::instrument(skip(app), err)]
pub async fn setup(app: tauri::AppHandle) -> Result<()> {
	info!("Trying to setup the application...");

	let state = app.state::<AppState>();
	state.initialize(&app).await?;

	info!("Setup was successful");
	Ok(())
}
