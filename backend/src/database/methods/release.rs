use bonsaidb::{
	core::schema::{SerializedCollection, SerializedView},
	local::AsyncDatabase,
};

use crate::{
	database::views::release::{ReleaseByNameAndArtist, ReleaseByNameAndArtistKey},
	errors::Result,
	models::temp::release::{TempRelease, TempReleaseIntoArg},
};

pub async fn update_or_insert(
	database: &AsyncDatabase,
	temp: TempRelease,
	library_id: u32,
	arg: TempReleaseIntoArg,
) -> Result<u64> {
	let artist_ids = arg.artists.iter().map(|x| x.id).collect::<Vec<u64>>();
	let keys = artist_ids
		.iter()
		.map(|x| ReleaseByNameAndArtistKey::new(temp.name.clone(), *x))
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
		if !document.contents.library_ids.contains(&library_id) {
			document.contents.library_ids.push(library_id);
			document.update_async(database).await?;
		}

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
