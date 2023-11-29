use bonsaidb::core::{
	document::{CollectionDocument, Emit},
	key::Key,
	schema::{CollectionMapReduce, ReduceResult, View, ViewMapResult, ViewMappedValue, ViewSchema},
};

use crate::database::models::resource::{Resource, ResourceType};

#[derive(Debug, Clone, PartialEq, Key)]
pub struct ResourceByTypeAndHashKey {
	pub type_: ResourceType,
	pub hash: String,
}

impl ResourceByTypeAndHashKey {
	pub fn new(type_: ResourceType, hash: String) -> Self {
		Self { type_, hash }
	}
}

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = Resource, key = ResourceByTypeAndHashKey, value = u64)]
pub struct ResourceByTypeAndHash;

impl CollectionMapReduce for ResourceByTypeAndHash {
	fn map<'doc>(&self, document: CollectionDocument<Resource>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		let key = ResourceByTypeAndHashKey::new(x.type_, x.hash.to_string());
		document.header.emit_key_and_value(key, 1)
	}

	fn reduce(&self, mappings: &[ViewMappedValue<Self>], _rereduce: bool) -> ReduceResult<Self::View> {
		Ok(mappings.iter().map(|m| m.value).sum())
	}
}
