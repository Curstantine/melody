use bonsaidb::core::{
	document::{BorrowedDocument, Emit},
	schema::{ReduceResult, SerializedCollection, View, ViewMapResult, ViewMappedValue, ViewSchema},
};

use crate::database::models::library::Library as LibraryModel;

#[derive(Debug, Clone, View)]
#[view(collection = LibraryModel, key = String, value = u32)]
pub struct LibraryByName;

impl ViewSchema for LibraryByName {
	type View = Self;

	fn map(&self, document: &BorrowedDocument<'_>) -> ViewMapResult<Self::View> {
		let library = LibraryModel::document_contents(document)?;
		document.header.emit_key_and_value(library.name, 1)
	}

	fn reduce(&self, mappings: &[ViewMappedValue<Self>], _rereduce: bool) -> ReduceResult<Self::View> {
		Ok(mappings.iter().map(|m| m.value).sum())
	}
}

#[cfg(test)]
mod test {
	use crate::{
		database::{models::library::Library as LibraryModel, views::library::LibraryByName, Database},
		errors::Result,
	};
	use bonsaidb::core::{connection::AsyncConnection, schema::SerializedCollection};

	#[tokio::test]
	async fn get_library_by_name() -> Result<()> {
		let memory_db = Database::testing().await?;
		let db = &memory_db.0;

		for i in 0..10 {
			let library = LibraryModel {
				name: format!("Library {}", i),
				scan_locations: vec![],
			};

			library.push_into_async(db).await?;
		}

		let doc_len = db.view::<LibraryByName>().reduce().await?;
		assert_eq!(doc_len, 10);

		let docs_with_5 = db
			.view::<LibraryByName>()
			.with_key("Library 5".to_string())
			.query_with_docs()
			.await?;

		assert_eq!(docs_with_5.len(), 1);

		Ok(())
	}
}
