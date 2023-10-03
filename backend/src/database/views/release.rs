use bonsaidb::{
	core::{
		document::{CollectionDocument, Emit, Header},
		schema::{
			view::map::Mappings, CollectionMapReduce, Map as BonsaiMap, SerializedCollection, SerializedView, View,
			ViewMapResult, ViewSchema,
		},
	},
	local::AsyncDatabase,
};
use tracing::debug;

use crate::{database::models::release::Release as ReleaseModel, errors::Result};

pub type ReleaseByNameAndArtistData = (String, Option<u64>);

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = ReleaseModel, key = ReleaseByNameAndArtistData, value = u64, name = "by-release-name-and-artist")]
pub struct ReleaseByNameAndArtist;

impl CollectionMapReduce for ReleaseByNameAndArtist {
	fn map<'doc>(&self, document: CollectionDocument<ReleaseModel>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;

		// TODO: Find a better way to handle multiple artists, multiple mappings for an entry.
		match &x.artists {
			Some(artists) => {
				let mut maps = Vec::<BonsaiMap<ReleaseByNameAndArtistData, u64>>::with_capacity(artists.len());
				let header = Header::try_from(document.header)?;

				for artist in artists {
					let key_tuple: ReleaseByNameAndArtistData = (x.name.clone(), Some(artist.id));
					maps.push(BonsaiMap::new(header.clone(), key_tuple, 1));
				}

				Ok(Mappings::List(maps))
			}
			None => document.header.emit_key_and_value((x.name, None), 1),
		}
	}
}

impl ReleaseByNameAndArtist {
	pub async fn put_or_get(database: &AsyncDatabase, release: ReleaseModel) -> Result<u64> {
		// There's definitely a better way to handle. Like to match a subset of artist ids instead of the whole array.
		let artist_ids = release
			.artists
			.as_ref()
			.map(|x| x.iter().map(|x| x.id).collect::<Vec<u64>>());

		let key_tuple: ReleaseByNameAndArtistData = (release.name.clone(), artist_ids.and_then(|x| x.first().copied()));
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

#[cfg(test)]
mod test {
	use bonsaidb::core::schema::{SerializedCollection, SerializedView};

	use crate::database::{
		models::{release::Release, InlinedArtist},
		views::release::ReleaseByNameAndArtist,
	};

	#[tokio::test]
	async fn test_release_by_artist_put_and_get() {
		let db = crate::database::Database::testing().await.unwrap();
		let database = db.0;

		// Release with an overlapping name, but no artists.
		let release_1 = Release::default();

		// Release with an overlapping name, but with artists.
		let release_2 = Release {
			artists: Some(vec![
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
			]),
			..Default::default()
		};

		// Different name, but with same artists.
		let release_3 = Release {
			name: "Test Release 2".to_string(),
			artists: Some(vec![
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
			]),
			..Default::default()
		};

		release_1.push_into_async(&database).await.unwrap();
		release_2.push_into_async(&database).await.unwrap();
		release_3.push_into_async(&database).await.unwrap();

		let see_dup_names_no_artist = ReleaseByNameAndArtist::entries_async(&database)
			.with_key(&("Test Release".to_string(), None))
			.query()
			.await
			.unwrap();

		assert_eq!(see_dup_names_no_artist.len(), 1);

		let see_dup_names_with_artist_1 = ReleaseByNameAndArtist::entries_async(&database)
			.with_key(&("Test Release".to_string(), Some(1)))
			.query()
			.await
			.unwrap();

		assert_eq!(see_dup_names_with_artist_1.len(), 1);

		let see_dup_names_with_artist_2 = ReleaseByNameAndArtist::entries_async(&database)
			.with_key(&("Test Release".to_string(), Some(2)))
			.query()
			.await
			.unwrap();

		assert_eq!(see_dup_names_with_artist_2.len(), 1);

		let new_name_with_artist_1 = ReleaseByNameAndArtist::entries_async(&database)
			.with_key(&("Test Release 2".to_string(), Some(1)))
			.query()
			.await
			.unwrap();

		assert_eq!(new_name_with_artist_1.len(), 1);
	}
}
