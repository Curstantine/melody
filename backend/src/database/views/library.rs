use bonsaidb::{
	core::{
		document::{CollectionDocument, Emit},
		schema::{
			CollectionMapReduce, ReduceResult, SerializedCollection, SerializedView, View, ViewMapResult,
			ViewMappedValue, ViewSchema,
		},
	},
	local::AsyncDatabase,
};
use tracing::debug;

use crate::{
	database::models::library::Library as LibraryModel,
	errors::{Error, Result},
};

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

impl LibraryByName {
	/// Sets a unique [LibraryModal] to the database
	pub async fn set_unique(database: &AsyncDatabase, library: LibraryModel) -> Result<()> {
		let matches = LibraryByName::entries_async(database)
			.with_key(&library.name)
			.query()
			.await?;

		if !matches.is_empty() {
			return Err(Error::descriptive("Library already exists"));
		}

		let lib = library.push_into_async(database).await?;
		debug!("Created library: {:#?} ({:?})", lib.contents, lib.header.id);

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::{
		database::{models::library::Library as LibraryModel, views::library::LibraryByName, Database},
		errors::Result,
	};
	use bonsaidb::core::schema::{SerializedCollection, SerializedView};

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

		let doc_len = LibraryByName::entries_async(db).reduce().await?;
		assert_eq!(doc_len, 10);

		let see_library_5 = LibraryByName::entries_async(db)
			.with_key(&"Library 5".to_string())
			.query()
			.await?;

		assert_eq!(see_library_5.len(), 1);

		Ok(())
	}
}
