use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Label {
	pub id: Uuid,
	pub name: String,
}

impl Label {
	pub fn temp(name: String) -> Self {
		Self { id: Uuid::nil(), name }
	}

	pub fn as_new(&mut self) -> &Self {
		self.id = Uuid::now_v7();
		self
	}
}
