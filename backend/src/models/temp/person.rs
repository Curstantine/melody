use crate::database::models::{
	person::{Person, PersonType},
	InlinedArtist,
};

#[derive(Debug)]
pub struct TempPerson {
	pub type_: PersonType,
	pub name: String,
	pub name_sort: Option<String>,
	pub mbz_id: Option<String>,
}

#[derive(Debug)]
pub struct TempInlinedArtist {
	pub credited_as: Option<String>,
	pub join: Option<String>,
}

#[derive(Debug)]
pub struct TempPersonCredit {
	pub person: TempPerson,
	pub inline: TempInlinedArtist,
}

impl TempPerson {
	pub fn into_person(self, library_ids: Vec<u32>) -> Person {
		Person {
			type_: self.type_,
			name: self.name,
			name_sort: self.name_sort,
			mbz_id: self.mbz_id,
			library_ids,
		}
	}
}

impl TempInlinedArtist {
	pub fn into_inlined(self, id: u64) -> InlinedArtist {
		InlinedArtist {
			id,
			credited_as: self.credited_as,
			join: self.join,
		}
	}
}

impl From<TempPerson> for TempPersonCredit {
	fn from(value: TempPerson) -> Self {
		TempPersonCredit {
			person: value,
			inline: TempInlinedArtist {
				credited_as: None,
				join: None,
			},
		}
	}
}
