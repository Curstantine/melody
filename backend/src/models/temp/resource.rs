use blake3::Hash;

use crate::database::models::resource::{Resource, ResourceMediaType, ResourceRelationType, ResourceType};

#[derive(Debug)]
pub struct TempResource {
	pub type_: ResourceType,
	pub relation_type: ResourceRelationType,
	pub media_type: ResourceMediaType,
	pub data: Box<[u8]>,
}

impl TempResource {
	pub fn into_resource(self, has_thumb: bool, hash: Hash) -> Resource {
		Resource {
			type_: self.type_,
			relation_type: self.relation_type,
			media_type: self.media_type,
			has_thumb,
			hash,
		}
	}
}
