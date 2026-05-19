use uuid::Uuid;

use crate::models::person::{InlinePerson, Person, PersonType};

#[derive(Debug)]
pub struct TempInlinePerson {
	pub person: Person,
	pub type_: PersonType,
}

impl TempInlinePerson {
	pub fn new(person: Person, type_: PersonType) -> Self {
		Self { person, type_ }
	}
}

impl TempInlinePerson {
	pub fn into_inlined(self, resource_id: Uuid) -> InlinePerson {
		InlinePerson {
			id: self.person.id,
			type_: self.type_,
			resource_id,
		}
	}
}
