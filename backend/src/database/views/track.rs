use bonsaidb::core::{
	document::{CollectionDocument, Emit},
	schema::{CollectionMapReduce, View, ViewMapResult, ViewSchema},
};

use crate::database::models::track::Track;

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = Track, key = u64, value = ())]
pub struct TrackByReleaseId;

impl CollectionMapReduce for TrackByReleaseId {
	fn map<'doc>(&self, document: CollectionDocument<Track>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;

		document.header.emit_key(x.release_id)
	}
}
