use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TagType {
	Genre,
	Other,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
	pub id: Uuid,
	pub name: String,

	#[serde(rename = "type")]
	pub type_: TagType,
}

impl Tag {
	pub fn temp(name: String, type_: TagType) -> Self {
		Self {
			id: Uuid::nil(),
			name,
			type_,
		}
	}

	pub fn as_new(&mut self) -> &Self {
		self.id = Uuid::now_v7();
		self
	}
}
