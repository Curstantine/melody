use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonType {
	Artist,
	Composer,
	Producer,

	/// Special type strictly for handling unknown people [Person::unknown]
	Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Person {
	pub id: u64,
	pub name: String,
	pub name_sort: Option<String>,
	pub mbz_id: Option<String>,

	#[serde(rename = "type")]
	pub type_: PersonType,
}

impl Person {
	pub const UNKNOWN_ID: u64 = 0;

	/// Create a [Person] that follows the default semantics for an unknown person.
	///
	/// See [super::InlinedArtist::unknown] for the inlined equivalent.
	pub fn unknown() -> Self {
		Self {
			id: Self::UNKNOWN_ID,
			name: "Unknown".to_string(),
			type_: PersonType::Unknown,
			name_sort: None,
			mbz_id: None,
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InlinePerson {
	pub id: u64,
	/// This is different from the [Person::name] field.
	///
	/// This field refers to an "alias" used by this [Person] in the context of the related entry.
	/// E.g. A person in release credit may use a different name than the one they use in the artist credit.
	pub credited_as: Option<String>,
	pub join: Option<String>,
}

impl InlinePerson {
	/// Create an [InlinedArtist] that follows the default semantics for an unknown artist.
	pub fn unknown() -> Self {
		Self {
			id: Person::UNKNOWN_ID,
			credited_as: None,
			join: None,
		}
	}
}
