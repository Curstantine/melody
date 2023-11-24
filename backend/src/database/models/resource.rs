use bonsaidb::core::schema::Collection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
	Artist,
	Release,
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "resources")]
pub struct Resource {
	pub type_: ResourceType,
	pub path: String,
}
