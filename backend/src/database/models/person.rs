use bonsaidb::core::{key::Key, schema::Collection};
use serde::{Deserialize, Serialize};

use crate::database::views::person::PersonByNameAndType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Key)]
pub enum PersonType {
	Artist,
	Composer,
	Producer,

	/// Special type strictly for handling unknown people (id 0).
	Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Collection)]
#[collection(name = "people", views = [PersonByNameAndType])]
pub struct Person {
	pub name: String,
	pub name_sort: Option<String>,
	pub mbz_id: Option<String>,

	#[serde(rename = "type")]
	pub type_: PersonType,
}

impl Person {
	/// Create a [Person] that follows the default semantics for an unknown person.
	///
	/// NOTE: Make sure this isn't used anywhere outside of testing and the initial database setup.
	///
	/// See [super::InlinedArtist::unknown] for the inlined equivalent.
	pub fn unknown() -> Self {
		Self {
			name: "Unknown".to_string(),
			type_: PersonType::Unknown,
			name_sort: None,
			mbz_id: None,
		}
	}
}

#[cfg(test)]
impl Default for Person {
	fn default() -> Self {
		Self {
			name: "Person".to_string(),
			type_: PersonType::Artist,
			name_sort: None,
			mbz_id: None,
		}
	}
}
