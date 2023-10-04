use bonsaidb::{
	core::{
		document::{CollectionDocument, Header},
		key::Key,
		schema::{
			view::map::Mappings, CollectionMapReduce, Map as BonsaiMap, SerializedCollection, SerializedView, View,
			ViewMapResult, ViewSchema,
		},
	},
	local::AsyncDatabase,
};
use tracing::debug;

use crate::{database::models::release::Release as ReleaseModel, errors::Result};

#[derive(Debug, Clone, PartialEq, Key)]
pub struct ReleaseByNameArtistKey {
	pub name: String,
	pub artist_id: u64,
}

impl ReleaseByNameArtistKey {
	pub fn new(name: String, artist_id: u64) -> Self {
		Self { name, artist_id }
	}
}

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = ReleaseModel, key = ReleaseByNameArtistKey, value = u8, name = "by-release-name-and-artist")]
pub struct ReleaseByNameAndArtist;

impl CollectionMapReduce for ReleaseByNameAndArtist {
	fn map<'doc>(&self, document: CollectionDocument<ReleaseModel>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;

		let mut maps = Vec::<BonsaiMap<ReleaseByNameArtistKey, u8>>::with_capacity(x.artists.len());
		let header = Header::try_from(document.header)?;

		for artist in x.artists {
			let key = ReleaseByNameArtistKey::new(x.name.clone(), artist.id);
			maps.push(BonsaiMap::new(header.clone(), key, 1));
		}

		Ok(Mappings::List(maps))
	}
}

impl ReleaseByNameAndArtist {
	pub async fn put_or_get(database: &AsyncDatabase, release: ReleaseModel) -> Result<u64> {
		let artist_ids = release.artists.iter().map(|x| x.id).collect::<Vec<u64>>();

		let key = ReleaseByNameArtistKey::new(release.name.clone(), *artist_ids.first().unwrap());
		let matches = ReleaseByNameAndArtist::entries_async(database)
			.with_key(&key)
			.query()
			.await?;

		let id = if matches.is_empty() {
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

#[cfg(test)]
mod test {
	use bonsaidb::core::schema::{SerializedCollection, SerializedView};

	use crate::{
		constants::{TEST_RELEASE_NAME, UNKNOWN_PERSON_ID},
		database::{
			models::{release::Release, InlinedArtist},
			views::release::{ReleaseByNameAndArtist, ReleaseByNameArtistKey},
		},
	};

	#[tokio::test]
	async fn test_release_by_artist_put_and_get() {
		let db = crate::database::Database::testing().await.unwrap();
		let database = db.0;

		// Release with "Test Release" name and DEFAULT_PERSON_ID artist.
		let release_1 = Release::default();
		let release_1_dup = Release::default();

		// Release with "Test Release" name and two artists new artists.
		let release_2 = Release {
			artists: vec![
				InlinedArtist {
					id: 1,
					credited_as: None,
					join: None,
				},
				InlinedArtist {
					id: 2,
					credited_as: None,
					join: None,
				},
			],
			..Default::default()
		};

		// Release with a different name as opposed to release_2, but with 1 artist in common.
		let release_3 = Release {
			name: "Test Release 2".to_string(),
			artists: vec![
				InlinedArtist {
					id: 1,
					credited_as: None,
					join: None,
				},
				InlinedArtist {
					id: 3,
					credited_as: None,
					join: None,
				},
			],
			..Default::default()
		};

		release_1.push_into_async(&database).await.unwrap();
		release_1_dup.push_into_async(&database).await.unwrap();
		release_2.push_into_async(&database).await.unwrap();
		release_3.push_into_async(&database).await.unwrap();

		let see_dup_names_no_artist = ReleaseByNameAndArtist::entries_async(&database)
			.with_key(&ReleaseByNameArtistKey::new(
				TEST_RELEASE_NAME.to_string(),
				UNKNOWN_PERSON_ID,
			))
			.query()
			.await
			.unwrap();

		assert_eq!(see_dup_names_no_artist.len(), 2);

		let dup_names_artist_1 = ReleaseByNameAndArtist::entries_async(&database)
			.with_key(&ReleaseByNameArtistKey::new(TEST_RELEASE_NAME.to_string(), 1))
			.query()
			.await
			.unwrap();

		assert_eq!(dup_names_artist_1.len(), 1);

		let dup_names_artist_2 = ReleaseByNameAndArtist::entries_async(&database)
			.with_key(&ReleaseByNameArtistKey::new(TEST_RELEASE_NAME.to_string(), 2))
			.query()
			.await
			.unwrap();

		assert_eq!(dup_names_artist_2.len(), 1);

		let new_name_artist_1 = ReleaseByNameAndArtist::entries_async(&database)
			.with_key(&ReleaseByNameArtistKey::new("Test Release 2".to_string(), 1))
			.query()
			.await
			.unwrap();

		assert_eq!(new_name_artist_1.len(), 1);
	}
}
