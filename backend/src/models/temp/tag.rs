use crate::database::models::tag::{Tag, TagType};

#[derive(Debug)]
pub struct TempTag {
	pub type_: TagType,
	pub name: String,
}

impl TempTag {
	pub fn into_tag(self, library_ids: Vec<u32>) -> Tag {
		Tag {
			name: self.name,
			type_: self.type_,
			library_ids,
		}
	}
}
