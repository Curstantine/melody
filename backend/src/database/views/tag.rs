use bonsaidb::{
	core::{
		document::{CollectionDocument, Emit},
		schema::{CollectionMapReduce, SerializedCollection, SerializedView, View, ViewMapResult, ViewSchema},
	},
	local::AsyncDatabase,
};
use tracing::debug;

use crate::{
	database::models::tag::{Tag as TagModel, TagType},
	errors::Result,
};

type TagByNameAndTypeData = (String, TagType);

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = TagModel, key = TagByNameAndTypeData, value = u64, name = "by-tag-name-and-type")]
pub struct TagByNameAndType;

impl CollectionMapReduce for TagByNameAndType {
	fn map<'doc>(&self, document: CollectionDocument<TagModel>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		document.header.emit_key_and_value((x.name, x.type_), 1)
	}
}

impl TagByNameAndType {
	pub async fn put_or_get(database: &AsyncDatabase, tag: TagModel) -> Result<u64> {
		let matches = TagByNameAndType::entries_async(database)
			.with_key(&(tag.name.clone(), tag.type_.clone()))
			.query()
			.await?;

		let id: u64;
		if matches.is_empty() {
			let label = tag.push_into_async(database).await?;
			id = label.header.id;
			debug!("Created tag: {:#?} ({:?})", label.contents, label.header.id);
		} else {
			let label = matches.first().unwrap();
			id = label.source.id;
			debug!("Found tag: {:#?} ({:?})", label.key, label.source.id);
		}

		Ok(id)
	}
}
