use std::borrow::Cow;

use bonsaidb::{
	core::{
		document::CollectionDocument,
		schema::{SerializedCollection, SerializedView},
	},
	local::AsyncDatabase,
};

use crate::{
	database::{
		models::person::Person,
		views::person::{PersonByNameAndType, PersonByNameAndTypeKey},
	},
	errors::{Error, Result},
	models::temp::person::TempPerson,
};

/// Inserts a document with a unique id.
///
/// The uniqueness is based solely on the id.
pub async fn insert_with_unique_id(
	database: &AsyncDatabase,
	person: Person,
	id: u64,
) -> Result<CollectionDocument<Person>> {
	match Person::get_async(&id, database).await? {
		Some(_) => {
			let m = format!("A person by this document id '{id}' already exists");
			let e = Error::new("Person already exists", Cow::Owned(m));
			Err(e)
		}
		None => person.insert_into_async(&id, database).await.map_err(Error::from),
	}
}

/// Either inserts a new entry or update a pre-existing person entry with the library_id.
///
/// Matched by [PersonByNameTypeKey]
pub async fn update_or_insert(database: &AsyncDatabase, temp: TempPerson, library_id: u32) -> Result<u64> {
	let key = PersonByNameAndTypeKey::new(temp.name.clone(), temp.type_.clone());
	let mut matches = PersonByNameAndType::entries_async(database)
		.with_key(&key)
		.limit(1)
		.query_with_collection_docs()
		.await?;

	let id = if let Some((id, mut document)) = matches.documents.pop_first() {
		document.contents.library_ids.push(library_id);
		document.update_async(database).await?;
		id
	} else {
		let doc = temp.into_person(vec![library_id]).push_into_async(database).await?;
		doc.header.id
	};

	Ok(id)
}
