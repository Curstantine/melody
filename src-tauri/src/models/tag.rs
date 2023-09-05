use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum TagType {
	Genre,
	Other,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
	pub name: String,

	#[serde(rename = "type")]
	pub type_: TagType,
}
