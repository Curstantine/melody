use crate::database::models::resource::{Resource, ResourceType};

#[derive(Debug)]
pub struct TempResource {
	pub type_: ResourceType,
	pub data: Box<[u8]>,
}

impl TempResource {
	pub fn into_resource(self, hash: String, path: String) -> Resource {
		Resource {
			type_: self.type_,
			hash,
			path,
		}
	}
}
