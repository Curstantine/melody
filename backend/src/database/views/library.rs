use bonsaidb::core::{
	document::{CollectionDocument, Emit},
	schema::{CollectionMapReduce, ReduceResult, View, ViewMapResult, ViewMappedValue, ViewSchema},
};

use crate::database::models::library::Library as LibraryModel;

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = LibraryModel, key = String, value = u32, name = "by-library-name")]
pub struct LibraryByName;

impl CollectionMapReduce for LibraryByName {
	fn map<'doc>(&self, document: CollectionDocument<LibraryModel>) -> ViewMapResult<'doc, Self::View> {
		document.header.emit_key_and_value(document.contents.name, 1)
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
	use bonsaidb::core::{
		connection::AsyncConnection,
		schema::{SerializedCollection, SerializedView},
	};

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

		let docs_with_5 = LibraryByName::entries_async(db)
			.with_key(&"Library 5".to_string())
			.query()
			.await?;

		assert_eq!(docs_with_5.len(), 1);

		Ok(())
	}
}
