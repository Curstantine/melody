// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use models::state::AppState;

mod commands;
mod constants;
mod database;
mod errors;
mod models;
mod utils;

fn main() {
	tauri::Builder::default()
		.manage(AppState::default())
		.invoke_handler(tauri::generate_handler![
			commands::general::setup,
			commands::library::create_library,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}