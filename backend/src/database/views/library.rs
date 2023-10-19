use bonsaidb::core::{
	document::{CollectionDocument, Emit},
	schema::{CollectionMapReduce, ReduceResult, View, ViewMapResult, ViewMappedValue, ViewSchema},
};

use crate::database::models::library::Library;

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = Library, key = String, value = u64)]
pub struct LibraryByName;

impl CollectionMapReduce for LibraryByName {
	fn map<'doc>(&self, document: CollectionDocument<Library>) -> ViewMapResult<'doc, Self::View> {
		document.header.emit_key_and_value(document.contents.name, 1)
	}

	fn reduce(&self, mappings: &[ViewMappedValue<Self>], _rereduce: bool) -> ReduceResult<Self::View> {
		Ok(mappings.iter().map(|m| m.value).sum())
	}
}
