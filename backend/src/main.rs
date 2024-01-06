// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::WindowEvent;
use tracing::info;

use crate::models::state::{AppState, DatabaseState, DirectoryState};

pub mod macros;

mod commands;
mod constants;
mod database;
mod errors;
mod ffmpeg;
mod models;
mod utils;

fn main() {
	tracing_subscriber::fmt::init();
	info!("Starting application");

	tauri::Builder::default()
		.on_window_event(|e| {
			// To alleviate the resize perf bugs mentioned in https://github.com/tauri-apps/tauri/issues/6322
			if let WindowEvent::Resized(_) = e.event() {
				std::thread::sleep(std::time::Duration::from_nanos(1));
			}
		})
		.manage(AppState::default())
		.manage(DirectoryState::default())
		.manage(DatabaseState::default())
		.invoke_handler(tauri::generate_handler![
			commands::general::setup,
			commands::library::create_library,
			commands::library::get_libraries,
			commands::release::get_releases,
			commands::release::get_display_releases,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
