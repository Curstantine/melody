use std::path::PathBuf;

use {tauri::PathResolver, tokio::fs};

use crate::{
	database::DB_MAIN_NAME,
	errors::{Error, Result},
};

pub struct Directories {
	pub database_dir: PathBuf,
	pub resource_cover_dir: PathBuf,
}

impl Directories {
	pub async fn new(path_resolver: PathResolver) -> Result<Self> {
		let data_dir = path_resolver.app_data_dir().expect("App data dir was not found");

		let database_dir = data_dir.join(DB_MAIN_NAME);
		let resource_dir = data_dir.join("resources");
		let cover_dir = resource_dir.join("covers");

		match fs::create_dir_all(&data_dir).await {
			Ok(_) => {
				fs::create_dir_all(&cover_dir)
					.await
					.map_err(|e| Error::from(e).set_path_data(cover_dir.clone()))?;
			}
			Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => {
				return Err(Error::from(e).set_path_data(data_dir));
			}
			_ => {}
		}

		Ok(Directories {
			database_dir,
			resource_cover_dir: cover_dir,
		})
	}
}
