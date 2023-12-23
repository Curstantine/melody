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
			let e = Error::new_dyn("Person already exists", Cow::Owned(m));
			Err(e)
		}
		None => person.insert_into_async(&id, database).await.map_err(Error::from),
	}
}

/// Inserts a document or gets an already existing one.
///
/// Uniqueness is based from name and type.
pub async fn get_or_insert(database: &AsyncDatabase, person: Person) -> Result<u64> {
	let key = PersonByNameAndTypeKey::new(person.name.clone(), person.type_.clone());
	let matches = PersonByNameAndType::entries_async(database)
		.with_key(&key)
		.query()
		.await?;

	let id = if let Some(person) = matches.first() {
		person.source.id
	} else {
		let person = person.push_into_async(database).await?;
		person.header.id
	};

	Ok(id)
}

#[cfg(test)]
mod test {
	use bonsaidb::core::schema::{SerializedCollection, SerializedView};

	use crate::{
		constants::UNKNOWN_PERSON_ID,
		database::{
			methods::person::{get_or_insert, insert_with_unique_id},
			models::person::{Person, PersonType},
			views::person::{PersonByNameAndType, PersonByNameAndTypeKey},
			Database,
		},
		errors::Result,
	};

	#[tokio::test]
	async fn test_insert_with_unique_id() -> Result<()> {
		let db = Database::testing().await?;
		let database = db.0;

		let person = Person::unknown();
		insert_with_unique_id(&database, person, UNKNOWN_PERSON_ID).await?;

		let person = Person::unknown();
		let result = insert_with_unique_id(&database, person, UNKNOWN_PERSON_ID).await;
		assert!(result.is_err());

		Ok(())
	}

	#[tokio::test]
	async fn test_get_or_insert() -> Result<()> {
		let db = Database::testing().await?;
		let dbx = db.0;

		let person = Person::unknown();
		let doc = insert_with_unique_id(&dbx, person, UNKNOWN_PERSON_ID).await?;
		assert_eq!(doc.header.id, UNKNOWN_PERSON_ID);

		let person = Person::unknown();
		let result = get_or_insert(&dbx, person).await?;
		assert_eq!(result, UNKNOWN_PERSON_ID);

		Ok(())
	}

	#[tokio::test]
	async fn test_by_name_and_type() -> Result<()> {
		let db = Database::testing().await?;
		let dbx = db.0;

		let person_1 = Person {
			name: "Person 1".to_string(),
			name_sort: Some("Person 1 Sort".to_string()),
			..Default::default()
		};

		let person_1_diff_sort = Person {
			name: "Person 1".to_string(),
			name_sort: Some("Person 1 Sort Different".to_string()),
			..Default::default()
		};

		let person_2 = Person {
			name: "Person 2".to_string(),
			name_sort: Some("Person 2 Sort".to_string()),
			..Default::default()
		};

		person_1.push_into_async(&dbx).await?;
		person_1_diff_sort.push_into_async(&dbx).await?;
		person_2.push_into_async(&dbx).await?;

		let see_person_1 = PersonByNameAndType::entries_async(&dbx)
			.with_key(&PersonByNameAndTypeKey::new("Person 1".to_string(), PersonType::Artist))
			.query()
			.await?;

		assert_eq!(see_person_1.len(), 2);

		let see_person_2 = PersonByNameAndType::entries_async(&dbx)
			.with_key(&PersonByNameAndTypeKey::new("Person 2".to_string(), PersonType::Artist))
			.query()
			.await?;

		assert_eq!(see_person_2.len(), 1);

		Ok(())
	}
}
