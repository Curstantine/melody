use bonsaidb::{
	core::{
		document::{CollectionDocument, Emit},
		schema::{CollectionMapReduce, SerializedCollection, SerializedView, View, ViewMapResult, ViewSchema},
	},
	local::AsyncDatabase,
};
use tracing::debug;

use crate::{database::models::track::Track as TrackModel, errors::Result};

pub type TrackByTitleAndReleaseData = (String, Option<u64>);

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = TrackModel, key = TrackByTitleAndReleaseData, value = u64, name = "by-track-title-and-release")]
pub struct TrackByTitleAndRelease;

impl CollectionMapReduce for TrackByTitleAndRelease {
	fn map<'doc>(&self, document: CollectionDocument<TrackModel>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		document.header.emit_key_and_value((x.title, x.release_id), 1)
	}
}

impl TrackByTitleAndRelease {
	pub async fn put_or_get(database: &AsyncDatabase, track: TrackModel) -> Result<u64> {
		let key_tuple: TrackByTitleAndReleaseData = (track.title.clone(), track.release_id);
		let matches = TrackByTitleAndRelease::entries_async(database)
			.with_key(&key_tuple)
			.query()
			.await?;

		let id: u64 = if matches.is_empty() {
			let track = track.push_into_async(database).await?;
			debug!("Created track: {:#?} ({:?})", track.contents, track.header.id);
			track.header.id
		} else {
			let track = matches.first().unwrap();
			debug!("Found track: {:#?} ({:?})", track.key, track.source.id);
			track.source.id
		};

		Ok(id)
	}
}
