use bonsaidb::{
	core::{
		document::{CollectionDocument, Emit},
		schema::{CollectionMapReduce, SerializedCollection, SerializedView, View, ViewMapResult, ViewSchema},
	},
	local::AsyncDatabase,
};

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

		let id = if matches.is_empty() {
			let label = person.push_into_async(database).await?;
			label.header.id
		} else {
			let label = matches.first().unwrap();
			label.source.id
		};

		Ok(id)
	}
}

#[cfg(test)]
mod test {
	use bonsaidb::core::schema::{SerializedCollection, SerializedView};

	use crate::{
		database::{models::label::Label, views::label::LabelByName, Database},
		errors::Result,
	};

	#[tokio::test]
	async fn test_label_by_name() -> Result<()> {
		let db = Database::testing().await?;
		let database = db.0;

		let label_1 = Label {
			name: "Label 1".to_string(),
		};

		let label_2 = Label {
			name: "Label 2".to_string(),
		};

		label_1.push_into_async(&database).await?;
		label_2.push_into_async(&database).await?;

		let see_label_1 = LabelByName::entries_async(&database)
			.with_key("Label 1")
			.query()
			.await
			.unwrap();

		assert_eq!(see_label_1.len(), 1);

		Ok(())
	}
}
