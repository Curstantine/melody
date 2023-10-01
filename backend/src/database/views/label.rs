use bonsaidb::{
	core::{
		document::{CollectionDocument, Emit},
		schema::{CollectionMapReduce, SerializedCollection, SerializedView, View, ViewMapResult, ViewSchema},
	},
	local::AsyncDatabase,
};
use tracing::debug;

use crate::{database::models::label::Label as LabelModel, errors::Result};

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = LabelModel, key = String, value = u64, name = "by-label-name")]
pub struct LabelByName;

impl CollectionMapReduce for LabelByName {
	fn map<'doc>(&self, document: CollectionDocument<LabelModel>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		document.header.emit_key_and_value(x.name, 1)
	}
}

impl LabelByName {
	pub async fn put_or_get(database: &AsyncDatabase, person: LabelModel) -> Result<u64> {
		let matches = LabelByName::entries_async(database)
			.with_key(&person.name)
			.query()
			.await?;

		let id: u64;
		if matches.is_empty() {
			let label = person.push_into_async(database).await?;
			id = label.header.id;
			debug!("Created label: {:#?} ({:?})", label.contents, label.header.id);
		} else {
			let label = matches.first().unwrap();
			id = label.source.id;
			debug!("Found label: {:#?} ({:?})", label.key, label.source.id);
		}

		Ok(id)
	}
}
