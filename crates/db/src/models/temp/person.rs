use uuid::Uuid;

use crate::models::person::{InlinePerson, Person};

#[derive(Debug)]
pub struct TempInlinePerson {
	pub person: Person,
	pub credited_as: Option<String>,
	pub join: Option<String>,
}

impl From<Person> for TempInlinePerson {
	fn from(person: Person) -> Self {
		Self {
			person,
			credited_as: None,
			join: None,
		}
	}
}

impl TempInlinePerson {
	pub fn into_inlined(self, id: Uuid) -> InlinePerson {
		InlinePerson {
			id,
			credited_as: self.credited_as,
			join: self.join,
		}
	}
}
