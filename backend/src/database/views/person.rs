use bonsaidb::core::{
	document::{CollectionDocument, Emit},
	key::Key,
	schema::{CollectionMapReduce, ReduceResult, View, ViewMapResult, ViewMappedValue, ViewSchema},
};

use crate::database::models::person::{Person, PersonType};

#[derive(Debug, Clone, PartialEq, Key)]
pub struct PersonByNameAndTypeKey {
	pub name: String,
	pub type_: PersonType,
}

impl PersonByNameAndTypeKey {
	pub fn new(name: String, type_: PersonType) -> Self {
		Self { name, type_ }
	}
}

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = Person, key = PersonByNameAndTypeKey, value = u64)]
pub struct PersonByNameAndType;

impl CollectionMapReduce for PersonByNameAndType {
	fn map<'doc>(&self, document: CollectionDocument<Person>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		let key = PersonByNameAndTypeKey::new(x.name, x.type_);
		document.header.emit_key_and_value(key, 1)
	}

	fn reduce(&self, mappings: &[ViewMappedValue<Self>], _rereduce: bool) -> ReduceResult<Self::View> {
		Ok(mappings.iter().map(|m| m.value).sum())
	}
}
