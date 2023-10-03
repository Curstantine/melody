use bonsaidb::core::schema::Collection;
use serde::{Deserialize, Serialize};

use crate::database::views::person::PersonByNameAndSort;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersonType {
	Artist,
	Composer,
	Producer,
}

#[derive(Debug, Clone, Serialize, Deserialize, Collection)]
#[collection(name = "people", views = [PersonByNameAndSort])]
pub struct Person {
	pub name: String,
	pub name_sort: Option<String>,
	pub mbz_id: Option<String>,

	#[serde(rename = "type")]
	pub type_: PersonType,
}
