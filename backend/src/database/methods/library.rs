use std::borrow::Cow;

use bonsaidb::{
	core::{
		document::CollectionDocument,
		schema::{SerializedCollection, SerializedView},
	},
	local::AsyncDatabase,
};

use crate::{
	database::{models::library::Library as LibraryModel, views::library::LibraryByName},
	errors::{Error, Result},
};

/// Inserts a unique [LibraryModal] to the database and returns an error when the document already exists.
///
/// Uniqueness is calculated from the name.
pub async fn insert_unique(
	database: &AsyncDatabase,
	library: LibraryModel,
) -> Result<CollectionDocument<LibraryModel>> {
	let matches = LibraryByName::entries_async(database)
		.with_key(&library.name)
		.query()
		.await?;

	if let Some(ex) = matches.first() {
		let m = format!("Library named '{}' already exists on the database.", ex.key);
		let e = Error::new_dyn("Library already exists", Cow::Owned(m));

		return Err(e);
	}

	library.push_into_async(database).await.map_err(Error::from)
}

#[cfg(test)]
mod test {
	use bonsaidb::core::schema::{SerializedCollection, SerializedView};

	use crate::{
		database::{
			methods::library::insert_unique, models::library::Library, views::library::LibraryByName, Database,
		},
		errors::Result,
	};

	#[tokio::test]
	async fn test_by_name() -> Result<()> {
		let db = Database::testing().await?;
		let dbx = db.0;

		for i in 0..10 {
			let library = Library {
				name: format!("Library {}", i),
				scan_locations: vec![],
			};

			library.push_into_async(&dbx).await?;
		}

		let len = LibraryByName::entries_async(&dbx).reduce().await?;
		assert_eq!(len, 10);

		let x = LibraryByName::entries_async(&dbx).with_key("Library 1").query().await?;
		assert_eq!(x.len(), 1);

		Ok(())
	}

	#[tokio::test]
	async fn test_insert_unique() -> Result<()> {
		let db = Database::testing().await?;
		let dbx = db.0;

		let library = Library {
			name: "Library 1".to_string(),
			scan_locations: vec![],
		};
		insert_unique(&dbx, library).await?;

		let library = Library {
			name: "Library 1".to_string(),
			scan_locations: vec![],
		};
		let result = insert_unique(&dbx, library).await;
		assert!(result.is_err());

		Ok(())
	}
}
