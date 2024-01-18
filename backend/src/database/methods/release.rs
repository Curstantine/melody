use bonsaidb::{
	core::schema::{SerializedCollection, SerializedView},
	local::AsyncDatabase,
};

use crate::{
	database::{
		models::release::Release,
		views::release::{ReleaseByNameAndArtist, ReleaseByNameAndArtistKey},
	},
	errors::Result,
	models::temp::release::{TempRelease, TempReleaseIntoArg},
};

/// Inserts a release or gets an already existing one.
///
/// Uniqueness is based on name and release artist id.
pub async fn get_or_insert(database: &AsyncDatabase, release: Release) -> Result<u64> {
	let artist_ids = release.artists.iter().map(|x| x.id).collect::<Vec<u64>>();

	let key = ReleaseByNameAndArtistKey::new(release.name.clone(), *artist_ids.first().unwrap());
	let matches = ReleaseByNameAndArtist::entries_async(database)
		.with_key(&key)
		.query()
		.await?;

	let id = if let Some(release) = matches.first() {
		release.source.id
	} else {
		let release = release.push_into_async(database).await?;
		release.header.id
	};

	Ok(id)
}

pub async fn update_or_insert(
	database: &AsyncDatabase,
	temp: TempRelease,
	library_id: u32,
	arg: TempReleaseIntoArg,
) -> Result<u64> {
	let artist_ids = arg.artists.iter().map(|x| x.id).collect::<Vec<u64>>();
	let keys = artist_ids
		.iter()
		.map(|x| ReleaseByNameAndArtistKey::new(temp.name.clone(), x.clone()))
		.collect::<Vec<_>>();

	let matches = ReleaseByNameAndArtist::entries_async(database)
		.with_keys(keys.as_slice())
		.limit(artist_ids.len() as u32)
		.query_with_collection_docs()
		.await?;

	let single_match = matches
		.documents
		.into_iter()
		.find(|(_, x)| x.contents.artists.iter().all(|x| artist_ids.contains(&x.id)));

	let id = if let Some((id, mut document)) = single_match {
		document.contents.library_ids.push(library_id);
		document.update_async(database).await?;
		id
	} else {
		let doc = temp
			.into_release(arg, vec![library_id])
			.push_into_async(database)
			.await?;
		doc.header.id
	};

	Ok(id)
}
