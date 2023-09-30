use bonsaidb::core::schema::Collection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum PersonType {
	Artist,
	Composer,
	Producer,
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "people")]
pub struct Person {
	pub id: String,
	pub name: String,
	pub name_sort: Option<String>,
	pub mbz_id: Option<String>,

	#[serde(rename = "type")]
	pub type_: PersonType,
}
