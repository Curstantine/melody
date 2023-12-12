use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::database::models::resource::Resource;

#[derive(Debug, Serialize)]
pub struct DisplayImageResource {
	#[serde(flatten)]
	pub inner: Resource,
	pub source_path: PathBuf,
	pub thumb_path: Option<PathBuf>,
}

impl DisplayImageResource {
	pub fn from_resource(resource: Resource, resource_cover_dir: &Path) -> Self {
		let hash_str = resource.hash.to_hex().to_string();
		let ext = resource.media_type.to_extension();

		let source_path = resource_cover_dir.join(format!("{}.{}", &hash_str, &ext));
		let thumb_path = resource
			.has_thumb
			.then(|| resource_cover_dir.join(format!("{}@512.{}", &hash_str, &ext)));

		Self {
			inner: resource,
			source_path,
			thumb_path,
		}
	}
}
