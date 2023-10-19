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

#[cfg(test)]
mod test {
	use bonsaidb::core::schema::{SerializedCollection, SerializedView};

	use crate::{
		constants::{TEST_RELEASE_NAME, UNKNOWN_PERSON_ID},
		database::{
			methods::release::get_or_insert,
			models::{release::Release, InlinedArtist},
			views::release::{ReleaseByNameAndArtist, ReleaseByNameAndArtistKey},
			Database,
		},
		errors::Result,
	};

	#[tokio::test]
	async fn test_get_or_insert() -> Result<()> {
		let db = Database::testing().await?;
		let dbx = db.0;

		let release = Release::default();
		let doc = release.insert_into_async(&1, &dbx).await?;
		assert_eq!(doc.header.id, 1);

		let release = Release::default();
		let result = get_or_insert(&dbx, release).await?;
		assert_eq!(result, 1);

		Ok(())
	}

	#[tokio::test]
	async fn test_by_name_and_artist() -> Result<()> {
		let db = Database::testing().await?;
		let dbx = db.0;

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

		release_1.push_into_async(&dbx).await.unwrap();
		release_1_dup.push_into_async(&dbx).await.unwrap();
		release_2.push_into_async(&dbx).await.unwrap();
		release_3.push_into_async(&dbx).await.unwrap();

		let see_dup_names_no_artist = ReleaseByNameAndArtist::entries_async(&dbx)
			.with_key(&ReleaseByNameAndArtistKey::new(
				TEST_RELEASE_NAME.to_string(),
				UNKNOWN_PERSON_ID,
			))
			.query()
			.await
			.unwrap();

		assert_eq!(see_dup_names_no_artist.len(), 2);

		let dup_names_artist_1 = ReleaseByNameAndArtist::entries_async(&dbx)
			.with_key(&ReleaseByNameAndArtistKey::new(TEST_RELEASE_NAME.to_string(), 1))
			.query()
			.await
			.unwrap();

		assert_eq!(dup_names_artist_1.len(), 1);

		let dup_names_artist_2 = ReleaseByNameAndArtist::entries_async(&dbx)
			.with_key(&ReleaseByNameAndArtistKey::new(TEST_RELEASE_NAME.to_string(), 2))
			.query()
			.await
			.unwrap();

		assert_eq!(dup_names_artist_2.len(), 1);

		let new_name_artist_1 = ReleaseByNameAndArtist::entries_async(&dbx)
			.with_key(&ReleaseByNameAndArtistKey::new("Test Release 2".to_string(), 1))
			.query()
			.await
			.unwrap();

		assert_eq!(new_name_artist_1.len(), 1);

		Ok(())
	}
}
