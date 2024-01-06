use {
	blake3::Hash,
	bonsaidb::core::{
		document::{CollectionDocument, Emit},
		key::Key,
		schema::{CollectionMapReduce, ReduceResult, View, ViewMapResult, ViewMappedValue, ViewSchema},
	},
};

use crate::database::models::cover::{Cover, CoverType};

#[derive(Debug, Clone, PartialEq, Key)]
pub struct CoverByTypeAndHashKey {
	pub type_: CoverType,
	pub hash: String,
}

impl CoverByTypeAndHashKey {
	pub fn new(type_: CoverType, hash: Hash) -> Self {
		Self {
			type_,
			hash: hash.to_hex().to_string(),
		}
	}
}

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = Cover, key = CoverByTypeAndHashKey, value = u64)]
pub struct CoverByTypeAndHash;

impl CollectionMapReduce for CoverByTypeAndHash {
	fn map<'doc>(&self, document: CollectionDocument<Cover>) -> ViewMapResult<'doc, Self::View> {
		let x = document.contents;
		let key = CoverByTypeAndHashKey::new(x.type_, x.hash);
		document.header.emit_key_and_value(key, 1)
	}

	fn reduce(&self, mappings: &[ViewMappedValue<Self>], _rereduce: bool) -> ReduceResult<Self::View> {
		Ok(mappings.iter().map(|m| m.value).sum())
	}
}
