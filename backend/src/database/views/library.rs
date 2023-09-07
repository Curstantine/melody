use bonsaidb::core::{
	document::{BorrowedDocument, Emit},
	schema::{SerializedCollection, View, ViewMapResult, ViewSchema},
};

use crate::database::models::library::Library as LibraryModel;

#[derive(Debug, View)]
#[view(collection = LibraryModel, key = String, value = u32)]
pub struct LibraryByName;

impl ViewSchema for LibraryByName {
	type View = Self;

	fn map(&self, document: &BorrowedDocument<'_>) -> ViewMapResult<Self::View> {
		let library = LibraryModel::document_contents(document)?;
		document.header.emit_key_and_value(library.name, 1)
	}
}
