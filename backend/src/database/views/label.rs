use bonsaidb::core::{
	document::{CollectionDocument, Emit},
	schema::{CollectionMapReduce, ReduceResult, View, ViewMapResult, ViewMappedValue, ViewSchema},
};

use crate::database::models::label::Label;

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = Label, key = String, value = u64)]
pub struct LabelByName;

impl CollectionMapReduce for LabelByName {
	fn map<'doc>(&self, document: CollectionDocument<Label>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		document.header.emit_key_and_value(x.name, 1)
	}

	fn reduce(&self, mappings: &[ViewMappedValue<Self>], _rereduce: bool) -> ReduceResult<Self::View> {
		Ok(mappings.iter().map(|m| m.value).sum())
	}
}
