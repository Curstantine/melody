use bonsaidb::core::{
	document::{CollectionDocument, Header},
	key::Key,
	schema::{view::map::Mappings, CollectionMapReduce, Map as BonsaiMap, View, ViewMapResult, ViewSchema},
};

use crate::database::models::release::Release;

#[derive(Debug, Clone, PartialEq, Key)]
pub struct ReleaseByNameAndArtistKey {
	pub name: String,
	pub artist_id: u64,
}

impl ReleaseByNameAndArtistKey {
	pub fn new(name: String, artist_id: u64) -> Self {
		Self { name, artist_id }
	}
}

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = Release, key = ReleaseByNameAndArtistKey, value = u8)]
pub struct ReleaseByNameAndArtist;

impl CollectionMapReduce for ReleaseByNameAndArtist {
	fn map<'doc>(&self, document: CollectionDocument<Release>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;

		let mut maps = Vec::<BonsaiMap<ReleaseByNameAndArtistKey, u8>>::with_capacity(x.artists.len());
		let header = Header::try_from(document.header)?;

		for artist in x.artists {
			let key = ReleaseByNameAndArtistKey::new(x.name.clone(), artist.id);
			maps.push(BonsaiMap::new(header.clone(), key, 1));
		}

		Ok(Mappings::List(maps))
	}
}
