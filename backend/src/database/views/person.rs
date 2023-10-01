use bonsaidb::{
	core::{
		document::{CollectionDocument, Emit},
		schema::{CollectionMapReduce, SerializedCollection, SerializedView, View, ViewMapResult, ViewSchema},
	},
	local::AsyncDatabase,
};
use tracing::debug;

use crate::{database::models::person::Person as PersonModel, errors::Result};

pub type PersonByNameAndSortData = (String, Option<String>);

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = PersonModel, key = PersonByNameAndSortData, value = u64, name = "by-person-name-and-sort")]
pub struct PersonByNameAndSort;

impl CollectionMapReduce for PersonByNameAndSort {
	fn map<'doc>(&self, document: CollectionDocument<PersonModel>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		document.header.emit_key_and_value((x.name, x.name_sort), 1)
	}
}

impl PersonByNameAndSort {
	pub async fn put_or_get(database: &AsyncDatabase, person: PersonModel) -> Result<u64> {
		let key_tuple: PersonByNameAndSortData = (person.name.clone(), person.name_sort.clone());
		let matches = PersonByNameAndSort::entries_async(database)
			.with_key(&key_tuple)
			.query()
			.await?;

		let id: u64;
		if matches.is_empty() {
			let person = person.push_into_async(database).await?;
			id = person.header.id;
			debug!("Created person: {:#?} ({:?})", person.contents, person.header.id);
		} else {
			// NOTE: Might need to check the probability of the match instead of just taking the first one.
			let person = matches.first().unwrap();
			id = person.source.id;
			debug!("Found person: {:#?} ({:?})", person.key, person.source.id);
		}

		Ok(id)
	}
}
