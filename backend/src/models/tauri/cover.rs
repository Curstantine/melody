use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::database::models::cover::Cover;

#[derive(Debug, Serialize)]
pub struct DisplayCoverResource {
	#[serde(flatten)]
	pub inner: Cover,
	pub source_path: PathBuf,
	pub thumb_path: Option<PathBuf>,
}

impl DisplayCoverResource {
	pub fn from_cover(resource: Cover, resource_cover_dir: &Path) -> Self {
		let hash_str = resource.hash.to_hex().to_string();
		let ext = resource.media_type.as_extension();

		let source_path = resource_cover_dir.join(format!("{hash_str}.{ext}"));
		let thumb_path = resource
			.has_thumb
			.then(|| resource_cover_dir.join(format!("{hash_str}@512.{ext}")));

		Self {
			inner: resource,
			source_path,
			thumb_path,
		}
	}
}
