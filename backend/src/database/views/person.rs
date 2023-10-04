use bonsaidb::{
	core::{
		document::{CollectionDocument, Emit},
		key::Key,
		schema::{CollectionMapReduce, SerializedCollection, SerializedView, View, ViewMapResult, ViewSchema},
	},
	local::AsyncDatabase,
};
use tracing::debug;

use crate::{
	database::models::person::{Person as PersonModel, PersonType},
	errors::Result,
};

#[derive(Debug, Clone, PartialEq, Key)]
pub struct PersonByNameAndTypeKey {
	pub name: String,
	pub type_: PersonType,
}

impl PersonByNameAndTypeKey {
	pub fn new(name: String, type_: PersonType) -> Self {
		Self { name, type_ }
	}
}

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = PersonModel, key = PersonByNameAndTypeKey, value = u8, name = "by-person-name-and-type")]
pub struct PersonByNameAndSort;

impl CollectionMapReduce for PersonByNameAndSort {
	fn map<'doc>(&self, document: CollectionDocument<PersonModel>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		let key = PersonByNameAndTypeKey::new(x.name, x.type_);
		document.header.emit_key_and_value(key, 1)
	}
}

impl PersonByNameAndSort {
	pub async fn put_or_get(database: &AsyncDatabase, person: PersonModel) -> Result<u64> {
		let key = PersonByNameAndTypeKey::new(person.name.clone(), person.type_.clone());
		let matches = PersonByNameAndSort::entries_async(database)
			.with_key(&key)
			.query()
			.await?;

		let id = if matches.is_empty() {
			let person = person.push_into_async(database).await?;
			debug!("Created person: {:#?} ({:?})", person.contents, person.header.id);
			person.header.id
		} else {
			// NOTE: Might need to check the probability of the match instead of just taking the first one.
			let person = matches.first().unwrap();
			debug!("Found person: {:#?} ({:?})", person.key, person.source.id);
			person.source.id
		};

		Ok(id)
	}
}

#[cfg(test)]
mod test {
	use bonsaidb::core::schema::{SerializedCollection, SerializedView};

	use crate::{
		database::{
			models::person::{Person, PersonType},
			views::person::{PersonByNameAndSort, PersonByNameAndTypeKey},
			Database,
		},
		errors::Result,
	};

	#[tokio::test]
	async fn test_person_by_name_and_sort() -> Result<()> {
		let db = Database::testing().await?;
		let database = db.0;

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

		person_1.push_into_async(&database).await?;
		person_1_diff_sort.push_into_async(&database).await?;
		person_2.push_into_async(&database).await?;

		let see_person_1 = PersonByNameAndSort::entries_async(&database)
			.with_key(&PersonByNameAndTypeKey::new("Person 1".to_string(), PersonType::Artist))
			.query()
			.await?;

		assert_eq!(see_person_1.len(), 2);

		let see_person_2 = PersonByNameAndSort::entries_async(&database)
			.with_key(&PersonByNameAndTypeKey::new("Person 2".to_string(), PersonType::Artist))
			.query()
			.await?;

		assert_eq!(see_person_2.len(), 1);

		Ok(())
	}
}
