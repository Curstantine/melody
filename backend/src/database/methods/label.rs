use bonsaidb::{
	core::schema::{SerializedCollection, SerializedView},
	local::AsyncDatabase,
};

use crate::{database::views::label::LabelByName, errors::Result, models::temp::label::TempLabel};

pub async fn update_or_insert(database: &AsyncDatabase, temp: TempLabel, library_id: u32) -> Result<u64> {
	let mut matches = LabelByName::entries_async(database)
		.with_key(&temp.name)
		.limit(1)
		.query_with_collection_docs()
		.await?;

	let id = if let Some((id, mut document)) = matches.documents.pop_first() {
		document.contents.library_ids.push(library_id);
		document.update_async(database).await?;
		id
	} else {
		let doc = temp.into_label(vec![library_id]).push_into_async(database).await?;
		doc.header.id
	};

	Ok(id)
}
