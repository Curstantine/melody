use {
	blake3::Hash,
	bonsaidb::{
		core::schema::{SerializedCollection, SerializedView},
		local::AsyncDatabase,
	},
};

use crate::{
	database::{
		models::cover::{Cover, CoverType},
		views::cover::{CoverByTypeAndHash, CoverByTypeAndHashKey},
	},
	errors::{pre::database_entry_not_found, Result},
};

pub async fn get_by_type_and_hash(database: &AsyncDatabase, type_: CoverType, hash: Hash) -> Result<Option<u64>> {
	let key = CoverByTypeAndHashKey::new(type_, hash);
	let matches = CoverByTypeAndHash::entries_async(database)
		.with_key(&key)
		.limit(1)
		.query()
		.await?;

	Ok(matches.first().map(|e| e.source.id))
}

/// Updates the [Cover::library_ids] vector with the passed [library_id]
pub async fn update_entry_lib_ids(database: &AsyncDatabase, cover_id: u64, library_id: u32) -> Result<()> {
	if let Some(mut cover) = Cover::get_async(&cover_id, database).await? {
		cover.contents.library_ids.push(library_id);
		cover.update_async(database).await?;
	} else {
		return Err(database_entry_not_found("covers", cover_id));
	}

	Ok(())
}
