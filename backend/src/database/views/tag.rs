use bonsaidb::core::{
	document::{CollectionDocument, Emit},
	key::Key,
	schema::{CollectionMapReduce, ReduceResult, View, ViewMapResult, ViewMappedValue, ViewSchema},
};

use crate::database::models::tag::{Tag, TagType};

#[derive(Debug, Clone, PartialEq, Key)]
pub struct TagByNameAndTypeKey {
	pub name: String,
	pub type_: TagType,
}

impl TagByNameAndTypeKey {
	pub fn new(name: String, type_: TagType) -> Self {
		Self { name, type_ }
	}
}

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = Tag, key = TagByNameAndTypeKey, value = u64)]
pub struct TagByNameAndType;

impl CollectionMapReduce for TagByNameAndType {
	fn map<'doc>(&self, document: CollectionDocument<Tag>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		let key = TagByNameAndTypeKey::new(x.name, x.type_);
		document.header.emit_key_and_value(key, 1)
	}

	fn reduce(&self, mappings: &[ViewMappedValue<Self>], _rereduce: bool) -> ReduceResult<Self::View> {
		Ok(mappings.iter().map(|m| m.value).sum())
	}
}
