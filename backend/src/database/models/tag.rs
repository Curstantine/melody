use bonsaidb::core::{key::Key, schema::Collection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Key)]
pub enum TagType {
	Genre,
	Other,
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "tags")]
pub struct Tag {
	pub name: String,

	#[serde(rename = "type")]
	pub type_: TagType,
}
