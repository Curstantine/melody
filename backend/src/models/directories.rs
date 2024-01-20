use std::path::{Path, PathBuf};

use {blake3::Hash, tauri::PathResolver};

use crate::{
	database::constants::DB_MAIN_NAME,
	errors::{Error, Result},
};

pub struct Directories {
	pub database_dir: PathBuf,
	pub cover_dir: PathBuf,
}

impl Directories {
	const COVER_FOLDER_NAME: &'static str = "covers";
	const THUMB_FOLDER_NAME: &'static str = "thumbs";

	#[inline]
	fn new(database_dir: PathBuf, cover_dir: PathBuf) -> Self {
		Self {
			database_dir,
			cover_dir,
		}
	}

	pub async fn initialize(path_resolver: PathResolver) -> Result<Self> {
		let data_dir = path_resolver.app_data_dir().expect("App data dir was not found");

		tokio::task::spawn_blocking::<_, Result<Directories>>(move || {
			let database_dir = data_dir.join(DB_MAIN_NAME);
			let cover_dir = data_dir.join(Self::COVER_FOLDER_NAME);

			std::fs::create_dir_all(&data_dir)
				.map_err(|e| Error::from(e).append_message("Failed to create 'data' directory"))?;

			std::fs::create_dir_all(&cover_dir)
				.map_err(|e| Error::from(e).append_message("Failed to create 'covers' directory"))?;

			std::fs::create_dir_all(cover_dir.join(Self::THUMB_FOLDER_NAME))
				.map_err(|e| Error::from(e).append_message("Failed to create 'covers/thumbs' directory"))?;

			Ok(Directories::new(database_dir, cover_dir))
		})
		.await?
	}
}

pub fn get_cover_path(cover_dir: &Path, hash: &Hash, extension: &str, is_thumb: bool) -> PathBuf {
	let hex = hash.to_hex();
	let x = format!("{}.{extension}", hex.as_str());

	if !is_thumb {
		cover_dir.join(x)
	} else {
		cover_dir.join(format!("{f}/{x}", f = Directories::THUMB_FOLDER_NAME))
	}
}
