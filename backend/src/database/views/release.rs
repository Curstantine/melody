// use bonsaidb::core::{
// 	document::{CollectionDocument, Emit},
// 	schema::{CollectionMapReduce, View, ViewMapResult, ViewSchema},
// };

// use crate::database::models::release::Release as ReleaseModel;

// #[derive(Debug, Clone, View, ViewSchema)]
// #[view(collection = ReleaseModel, key = String, value = Option<Vec<String>>, name = "by-release-name-and-artists")]
// pub struct ReleaseByNameAndReleaseArtistIds;

// impl CollectionMapReduce for ReleaseByNameAndReleaseArtistIds {
// 	fn map<'doc>(&self, document: CollectionDocument<ReleaseModel>) -> ViewMapResult<'doc, Self::View> {
// 		document
// 			.header
// 			.emit_key_and_value(document.contents.name, document.contents.artist_ids)
// 	}
// }
