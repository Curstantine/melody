use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{database::models::cover::Cover, models::directories::get_cover_path};

#[derive(Debug, Serialize)]
pub struct DisplayCoverResource {
	#[serde(flatten)]
	pub inner: Cover,
	pub path: PathBuf,
}

impl DisplayCoverResource {
	pub fn from_cover(cover: Cover, cover_dir: &Path) -> Self {
		Self {
			path: get_cover_path(cover_dir, &cover.hash, cover.media_type.as_extension(), cover.has_thumb),
			inner: cover,
		}
	}
}
