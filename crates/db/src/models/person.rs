use serde::{Deserialize, Serialize};
use uuid::{Uuid, uuid};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonType {
	Artist,
	Composer,
	Producer,
	Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Person {
	pub id: Uuid,
	pub name: String,
	pub name_sort: Option<String>,
	pub mbz_id: Option<String>,
}

impl Person {
	pub const UNKNOWN_ID: Uuid = uuid!("00000000-0000-0000-0000-ffff00000001");

	pub fn temp(name: String, name_sort: Option<String>, mbz_id: Option<String>) -> Self {
		Self {
			id: Uuid::nil(),
			name,
			name_sort,
			mbz_id,
		}
	}

	/// Create a [Person] that follows the default semantics for an unknown person.
	///
	/// See [super::InlinedArtist::unknown] for the inlined equivalent.
	pub fn unknown() -> Self {
		Self {
			id: Self::UNKNOWN_ID,
			name: "Unknown".to_string(),
			name_sort: None,
			mbz_id: None,
		}
	}

	pub fn as_new(&mut self) -> &Self {
		self.id = Uuid::now_v7();
		self
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InlinePerson {
	pub id: Uuid,
	pub resource_id: Uuid,
	#[serde(rename = "type")]
	pub type_: PersonType,
}

impl InlinePerson {
	/// Create an [InlinedArtist] that follows the default semantics for an unknown artist.
	pub fn unknown() -> Self {
		Self {
			id: Person::UNKNOWN_ID,
			resource_id: Uuid::nil(),
			type_: PersonType::Unknown,
		}
	}
}
