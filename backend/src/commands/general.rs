use {tauri::Manager, tracing::info};

use rsmpeg::ffi::{av_log_set_level, AV_LOG_FATAL};
use tauri::AppHandle;

use crate::{
	errors::Result,
	models::state::{AppState, DatabaseState, DirectoryState},
};

#[tauri::command]
#[tracing::instrument(skip(app), err)]
pub async fn setup(app: AppHandle) -> Result<()> {
	info!("Setting up the application...");

	let app_state = app.state::<AppState>();
	let dir_state = app.state::<DirectoryState>();
	let db_state = app.state::<DatabaseState>();

	if let Err(()) = app_state.initialize() {
		return Ok(());
	}

	let path_resolver = app.path_resolver();
	dir_state.initialize(path_resolver).await?;

	let database_dir = {
		let guard = dir_state.get();
		let directories = guard.as_ref().unwrap();
		directories.database_dir.clone()
	};
	db_state.initialize(&database_dir).await?;

	unsafe {
		av_log_set_level(AV_LOG_FATAL.try_into().unwrap());
	}

	info!("Setup was successful");

	Ok(())
}
