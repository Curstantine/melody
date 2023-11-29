use std::path::Path;

use bonsaidb::{
	core::schema::{SerializedCollection, SerializedView},
	local::AsyncDatabase,
};

use crate::{
	database::{
		models::resource::ResourceType,
		views::resource::{ResourceByTypeAndHash, ResourceByTypeAndHashKey},
	},
	errors::{Error, FromErrorWithContextData, IoErrorType, Result},
	models::temp::resource::TempResource,
};

pub async fn get_or_insert(database: &AsyncDatabase, resource_cover_dir: &Path, resource: TempResource) -> Result<u64> {
	let hash = blake3::hash(&resource.data);
	let hash_str = hash.to_hex();

	let key = ResourceByTypeAndHashKey::new(ResourceType::Release, hash_str.to_string());
	let matches = ResourceByTypeAndHash::entries_async(database)
		.with_key(&key)
		.limit(1)
		.query()
		.await?;

	if let Some(entry) = matches.get(0) {
		return Ok(entry.value);
	}

	let path = resource_cover_dir.join(format!("{}.{}", hash_str, resource.media_type.to_extension()));
	tokio::fs::write(&path, &resource.data)
		.await
		.map_err(|e| Error::from_with_ctx(e, IoErrorType::Path(resource_cover_dir)))?;

	let res = resource.into_resource(hash, path);
	let doc = res.push_into_async(database).await?;

	Ok(doc.header.id)
}
