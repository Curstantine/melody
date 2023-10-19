use bonsaidb::{
	core::schema::{SerializedCollection, SerializedView},
	local::AsyncDatabase,
};

use crate::{
	database::{
		models::tag::Tag,
		views::tag::{TagByNameAndType, TagByNameAndTypeKey},
	},
	errors::Result,
};

/// Inserts a tag or gets an already existing one.
///
/// Uniqueness is based on name and tag type.
pub async fn get_or_insert(database: &AsyncDatabase, tag: Tag) -> Result<u64> {
	let key = TagByNameAndTypeKey::new(tag.name.clone(), tag.type_.clone());
	let matches = TagByNameAndType::entries_async(database).with_key(&key).query().await?;

	let id = if let Some(tag) = matches.first() {
		tag.source.id
	} else {
		let tag = tag.push_into_async(database).await?;
		tag.header.id
	};

	Ok(id)
}

#[cfg(test)]
mod test {
	use bonsaidb::core::schema::{SerializedCollection, SerializedView};

	use crate::{
		database::{
			methods::tag::get_or_insert,
			models::tag::{Tag, TagType},
			views::tag::{TagByNameAndType, TagByNameAndTypeKey},
			Database,
		},
		errors::Result,
	};

	#[tokio::test]
	async fn test_get_or_insert() -> Result<()> {
		let db = Database::testing().await?;
		let dbx = db.0;

		let tag = Tag {
			name: "Tag 1".to_string(),
			type_: TagType::Genre,
		};
		let doc = tag.push_into_async(&dbx).await?;

		let tag = Tag {
			name: "Tag 1".to_string(),
			type_: TagType::Genre,
		};
		let result = get_or_insert(&dbx, tag).await?;
		assert_eq!(result, doc.header.id);

		Ok(())
	}

	#[tokio::test]
	async fn test_by_name_and_type() -> Result<()> {
		let db = Database::testing().await?;
		let dbx = db.0;

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

		tag_1.push_into_async(&dbx).await?;
		tag_2.push_into_async(&dbx).await?;
		tag_1_other.push_into_async(&dbx).await?;

		let see_tag_1 = TagByNameAndType::entries_async(&dbx)
			.with_key(&TagByNameAndTypeKey::new("Tag 1".to_string(), TagType::Genre))
			.query()
			.await?;

		assert_eq!(see_tag_1.len(), 1);

		let see_tag_2 = TagByNameAndType::entries_async(&dbx)
			.with_key(&TagByNameAndTypeKey::new("Tag 2".to_string(), TagType::Genre))
			.query()
			.await?;

		assert_eq!(see_tag_2.len(), 1);

		let see_tag_1_other = TagByNameAndType::entries_async(&dbx)
			.with_key(&TagByNameAndTypeKey::new("Tag 1".to_string(), TagType::Other))
			.query()
			.await?;

		assert_eq!(see_tag_1_other.len(), 1);

		Ok(())
	}
}
