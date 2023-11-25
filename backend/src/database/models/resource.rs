use bonsaidb::core::schema::Collection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
	Artist,
	Release(ResourceReleaseType),
}

pub enum ResourceReleaseType {
	/// Front cover of a release or track.
	///
	/// This type will be unique for each release.
	FrontCover,

	/// Cover art unique to each track of a release.
	///
	/// Typically this is different from FrontCover.
	Track,

	Other(String),
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "resources")]
pub struct Resource {
	pub type_: ResourceType,
	pub path: String,
	pub hash: String,
}
