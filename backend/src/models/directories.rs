use std::path::PathBuf;

use tauri::AppHandle;

use crate::{
	database::DB_MAIN_NAME,
	errors::{Error, FromErrorWithContextData, IoErrorType, Result},
};

pub struct Directories {
	pub database_dir: PathBuf,
	pub resource_cover_dir: PathBuf,
}

impl Directories {
	pub async fn new(app_handle: &AppHandle) -> Result<Self> {
		let data_dir = app_handle
			.path_resolver()
			.app_data_dir()
			.expect("App data dir was not found");

		let database_dir = data_dir.join(DB_MAIN_NAME);
		let resource_dir = data_dir.join("resources");
		let resource_cover_dir = resource_dir.join("covers");

		match tokio::fs::create_dir_all(&data_dir).await {
			Ok(_) => {
				tokio::fs::create_dir_all(&resource_cover_dir)
					.await
					.map_err(|e| Error::from_with_ctx(e, IoErrorType::Path(&resource_cover_dir)))?;
			}
			Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => {
				return Err(Error::from_with_ctx(e, IoErrorType::Path(&data_dir)));
			}
			_ => {}
		}

		Ok(Directories {
			database_dir,
			resource_cover_dir,
		})
	}
}
