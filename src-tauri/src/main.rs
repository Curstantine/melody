// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{App, Manager, State};

use state::AppState;

mod commands;
mod constants;
mod database;
mod errors;
mod models;
mod state;
mod utils;

fn main() {
	tauri::Builder::default()
		.manage(AppState::default())
		.setup(setup)
		.invoke_handler(tauri::generate_handler![commands::library::create_library,])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}

fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
	let app_state: State<AppState> = app.state();
	let app_handle = app.handle();

	app_state.initialize(&app_handle)?;

	Ok(())
}
