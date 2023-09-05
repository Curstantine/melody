use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum PersonType {
	Artist,
	Composer,
	Producer,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Person {
	pub name: String,
	pub name_sort: Option<String>,

	#[serde(rename = "type")]
	pub type_: PersonType,
}
