// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use {
	tauri::{Manager, WindowEvent},
	tracing::info,
	window_shadows::set_shadow,
};

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
		.setup(|app| {
			let window = app.get_window("main").unwrap();

			#[cfg(any(windows, target_os = "macos"))]
			set_shadow(&window, true).expect("Unsupported platform!");

			Ok(())
		})
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
			commands::library::get_scan_locations,
			commands::library::initialize_library,
			commands::release::get_releases,
			commands::release::get_display_releases,
			commands::track::get_tracks_for_release,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
