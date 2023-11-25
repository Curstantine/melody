use crate::database::models::resource::{Resource, ResourceMediaType, ResourceType};

#[derive(Debug)]
pub struct TempResource {
	pub type_: ResourceType,
	pub media_type: ResourceMediaType,
	pub data: Box<[u8]>,
}

impl TempResource {
	pub fn into_resource(self, hash: String, path: String) -> Resource {
		Resource {
			type_: self.type_,
			media_type: self.media_type,
			hash,
			path,
		}
	}
}
