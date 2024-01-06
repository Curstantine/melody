use {
	blake3::Hash,
	bonsaidb::{core::schema::SerializedView, local::AsyncDatabase},
};

use crate::{
	database::{
		models::cover::CoverType,
		views::cover::{CoverByTypeAndHash, CoverByTypeAndHashKey},
	},
	errors::Result,
};

pub async fn get_id(database: &AsyncDatabase, type_: CoverType, hash: Hash) -> Result<Option<u64>> {
	let key = CoverByTypeAndHashKey::new(type_, hash);
	let matches = CoverByTypeAndHash::entries_async(database)
		.with_key(&key)
		.limit(1)
		.query()
		.await?;

	Ok(matches.get(0).map(|e| e.source.id))
}
