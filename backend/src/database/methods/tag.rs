use bonsaidb::{
	core::schema::{SerializedCollection, SerializedView},
	local::AsyncDatabase,
};

use crate::{
	database::views::tag::{TagByNameAndType, TagByNameAndTypeKey},
	errors::Result,
	models::temp::tag::TempTag,
};

pub async fn update_or_insert(database: &AsyncDatabase, temp: TempTag, library_id: u32) -> Result<u64> {
	let key = TagByNameAndTypeKey::new(temp.name.clone(), temp.type_.clone());
	let mut matches = TagByNameAndType::entries_async(database)
		.with_key(&key)
		.limit(1)
		.query_with_collection_docs()
		.await?;

	let id = if let Some((id, mut document)) = matches.documents.pop_first() {
		document.contents.library_ids.push(library_id);
		document.update_async(database).await?;
		id
	} else {
		let doc = temp.into_tag(vec![library_id]).push_into_async(database).await?;
		doc.header.id
	};

	Ok(id)
}
