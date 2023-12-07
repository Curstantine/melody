use blake3::Hash;
use bonsaidb::{core::schema::SerializedView, local::AsyncDatabase};

use crate::{
	database::{
		models::resource::{ResourceRelationType, ResourceType},
		views::resource::{ResourceByTypeAndHash, ResourceByTypeAndHashKey},
	},
	errors::Result,
};

pub async fn get_id(
	database: &AsyncDatabase,
	type_: ResourceType,
	relation_type: ResourceRelationType,
	hash: Hash,
) -> Result<Option<u64>> {
	let key = ResourceByTypeAndHashKey::new(type_, relation_type, hash);
	let matches = ResourceByTypeAndHash::entries_async(database)
		.with_key(&key)
		.limit(1)
		.query()
		.await?;

	Ok(matches.get(0).map(|e| e.source.id))
}
