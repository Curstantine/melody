use bonsaidb::{
	core::{
		document::{CollectionDocument, Emit},
		schema::{CollectionMapReduce, SerializedCollection, SerializedView, View, ViewMapResult, ViewSchema},
	},
	local::AsyncDatabase,
};
use tracing::debug;

use crate::{database::models::release::Release as ReleaseModel, errors::Result};

pub type ReleaseByNameAndArtistData = (String, Option<Vec<u64>>);

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = ReleaseModel, key = ReleaseByNameAndArtistData, value = u64, name = "by-release-name-and-artist")]
pub struct ReleaseByNameAndArtist;

impl CollectionMapReduce for ReleaseByNameAndArtist {
	fn map<'doc>(&self, document: CollectionDocument<ReleaseModel>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		let artist_ids = x.artists.map(|x| x.iter().map(|x| x.id).collect());
		document.header.emit_key_and_value((x.name, artist_ids), 1)
	}
}

impl ReleaseByNameAndArtist {
	pub async fn put_or_get(database: &AsyncDatabase, release: ReleaseModel) -> Result<u64> {
        // There's definitely a better way to handle. Like to match a subset of artist ids instead of the whole array.
        let artist_ids = release.artists.map(|x| x.iter().map(|x| x.id).collect());
		let key_tuple: ReleaseByNameAndArtistData = (release.name.clone(), artist_ids);
		let matches = ReleaseByNameAndArtist::entries_async(database)
			.with_key(&key_tuple)
			.query()
			.await?;

		let id: u64 = if matches.is_empty() {
			let release = release.push_into_async(database).await?;
			debug!("Created release: {:#?} ({:?})", release.contents, release.header.id);
			release.header.id
		} else {
			let release = matches.first().unwrap();
			debug!("Found release: {:#?} ({:?})", release.key, release.source.id);
			release.source.id
		};

		Ok(id)
	}
}
