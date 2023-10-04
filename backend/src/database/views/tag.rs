use bonsaidb::{
	core::{
		document::{CollectionDocument, Emit},
		key::Key,
		schema::{CollectionMapReduce, SerializedCollection, SerializedView, View, ViewMapResult, ViewSchema},
	},
	local::AsyncDatabase,
};

use crate::{
	database::models::tag::{Tag as TagModel, TagType},
	errors::Result,
};

#[derive(Debug, Clone, PartialEq, Key)]
pub struct TagByNameAndTypeKey {
	pub name: String,
	pub type_: TagType,
}

impl TagByNameAndTypeKey {
	pub fn new(name: String, type_: TagType) -> Self {
		Self { name, type_ }
	}
}

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = TagModel, key = TagByNameAndTypeKey, value = u64, name = "by-tag-name-and-type")]
pub struct TagByNameAndType;

impl CollectionMapReduce for TagByNameAndType {
	fn map<'doc>(&self, document: CollectionDocument<TagModel>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		let key = TagByNameAndTypeKey::new(x.name, x.type_);
		document.header.emit_key_and_value(key, 1)
	}
}

impl TagByNameAndType {
	pub async fn put_or_get(database: &AsyncDatabase, tag: TagModel) -> Result<u64> {
		let key = TagByNameAndTypeKey::new(tag.name.clone(), tag.type_.clone());
		let matches = TagByNameAndType::entries_async(database).with_key(&key).query().await?;

		let id = if matches.is_empty() {
			let label = tag.push_into_async(database).await?;
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
		database::{
			models::tag::{Tag, TagType},
			views::tag::{TagByNameAndType, TagByNameAndTypeKey},
			Database,
		},
		errors::Result,
	};

	#[tokio::test]
	async fn test_person_by_name_and_sort() -> Result<()> {
		let db = Database::testing().await?;
		let database = db.0;

		let tag_1 = Tag {
			name: "Tag 1".to_string(),
			type_: TagType::Genre,
		};

		let tag_2 = Tag {
			name: "Tag 2".to_string(),
			type_: TagType::Genre,
		};

		let tag_1_other = Tag {
			name: "Tag 1".to_string(),
			type_: TagType::Other,
		};

		tag_1.push_into_async(&database).await?;
		tag_2.push_into_async(&database).await?;
		tag_1_other.push_into_async(&database).await?;

		let see_tag_1 = TagByNameAndType::entries_async(&database)
			.with_key(&TagByNameAndTypeKey::new("Tag 1".to_string(), TagType::Genre))
			.query()
			.await?;

		assert_eq!(see_tag_1.len(), 1);

		let see_tag_2 = TagByNameAndType::entries_async(&database)
			.with_key(&TagByNameAndTypeKey::new("Tag 2".to_string(), TagType::Genre))
			.query()
			.await?;

		assert_eq!(see_tag_2.len(), 1);

		let see_tag_1_other = TagByNameAndType::entries_async(&database)
			.with_key(&TagByNameAndTypeKey::new("Tag 1".to_string(), TagType::Other))
			.query()
			.await?;

		assert_eq!(see_tag_1_other.len(), 1);

		Ok(())
	}
}
