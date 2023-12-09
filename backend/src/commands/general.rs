use {tauri::Manager, tracing::info};

use tauri::AppHandle;

use crate::{
	errors::Result,
	models::state::{AppState, DatabaseState, DirectoryState},
};

#[tauri::command]
#[tracing::instrument(skip(app), err)]
pub async fn setup(app: AppHandle) -> Result<()> {
	info!("Setting up the application...");

	let path_resolver = app.path_resolver();

	let app_state = app.state::<AppState>();
	let dir_state = app.state::<DirectoryState>();
	let db_state = app.state::<DatabaseState>();

	app_state.initialize();
	dir_state.initialize(path_resolver).await?;

	let guard = dir_state.get().await;
	let directories = guard.as_ref().unwrap();
	db_state.initialize(&directories.database_dir).await?;

	info!("Setup was successful");

	Ok(())
}
