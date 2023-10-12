use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use bonsaidb::core::schema::SerializedView;
use tokio::time::Instant;
use tracing::{debug, info};

use crate::database::helpers::handle_temp_track_meta;
use crate::utils::matchers;
use crate::{
	database::{models::library::Library as LibraryModel, views::library::LibraryByName},
	errors::Result,
	models::{
		state::AppState,
		tauri::{
			library::{LibraryActionType, LibraryEvent, LibraryGenericActionPayload},
			WindowEvent,
		},
		temp::TempTrackMeta,
	},
	utils::{self, symphonia::read_track_meta},
};

#[tauri::command]
#[tracing::instrument(skip(app_state))]
pub async fn get_library_names(app_state: tauri::State<'_, AppState>) -> Result<Vec<String>> {
	let db_lock = app_state.db.lock().await;
	let database = db_lock.as_ref().unwrap();
	let database = &database.0;

	let libraries = LibraryByName::entries_async(database).query_with_docs().await?;
	let names = libraries.into_iter().map(|x| x.key.clone()).collect::<Vec<_>>();

	Ok(names)
}

#[tauri::command]
#[tracing::instrument(skip(window, app_state), err(Debug))]
pub async fn create_library(
	name: String,
	scan_locations: Vec<String>,
	window: tauri::Window,
	app_state: tauri::State<'_, AppState>,
) -> Result<()> {
	let start = Instant::now();

	let db_lock = app_state.db.lock().await;
	let database = db_lock.as_ref().unwrap();
	let database = &database.0;

	let library = LibraryModel::new(name.clone(), scan_locations.clone());
	LibraryByName::set_unique(database, library).await?;

	enum ChannelData {
		ScanEvent(LibraryGenericActionPayload),
		ProbeResult {
			path: PathBuf,
			meta: Box<TempTrackMeta>,
			current: u64,
			total: u64,
		},
	}

	let (tx, rx) = mpsc::channel::<ChannelData>();
	let scan_location = scan_locations.into_iter().map(PathBuf::from).collect::<Vec<_>>();

	let handle = thread::spawn::<_, Result<()>>(move || {
		for scan_location in scan_location {
			let paths = utils::fs::walkdir_sync(scan_location, matchers::path::audio)?;
			let total = paths.len() as u64;

			for (i, path) in paths.into_iter().enumerate() {
				let current = i as u64 + 1;
				let extension = match path.extension() {
					Some(extension) => extension.to_str().unwrap().to_string(),
					_ => continue,
				};

				let event_payload = LibraryGenericActionPayload {
					total,
					current,
					action_type: LibraryActionType::Reading,
					path: path.clone(),
				};
				tx.send(ChannelData::ScanEvent(event_payload)).unwrap();

				let src = fs::File::open(&path)?;
				let meta = read_track_meta(Box::new(src), Some(&extension))?;
				tx.send(ChannelData::ProbeResult {
					meta: Box::new(meta),
					path,
					current,
					total,
				})
				.unwrap();
			}
		}

		Ok(())
	});

	for message in rx {
		match message {
			ChannelData::ScanEvent(payload) => {
				debug!(
					"Reading [{}/{}], currently reading:\n{:#?}",
					payload.current, payload.total, payload.path
				);
				WindowEvent::new(LibraryEvent::Scan, payload).emit(&window)?;
			}
			ChannelData::ProbeResult {
				total,
				current,
				path,
				meta,
			} => {
				debug!("Probed [{current}/{total}], currently indexing:\n{:#?}", path);
				WindowEvent::new(
					LibraryEvent::Scan,
					LibraryGenericActionPayload {
						total,
						current,
						action_type: LibraryActionType::Indexing,
						path,
					},
				)
				.emit(&window)?;

				handle_temp_track_meta(database, *meta).await?;
			}
		};
	}

	handle.join().unwrap()?;

	let elapsed = start.elapsed();
	info!("Finished building library in {:?}", elapsed);

	Ok(())
}
