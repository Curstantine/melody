use blake3::Hash;

use crate::database::models::cover::{Cover, CoverMediaType, CoverType};

pub struct TempCover {
	pub type_: CoverType,
	pub media_type: CoverMediaType,
	pub resolution: (u16, u16),
	pub comment: Option<String>,
	pub data: Box<[u8]>,
}

impl TempCover {
	pub fn into_cover(self, hash: Hash) -> Cover {
		Cover {
			type_: self.type_,
			media_type: self.media_type,
			resolution: self.resolution,
			comment: self.comment,
			hash,
		}
	}
}

impl std::fmt::Debug for TempCover {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("TempCover")
			.field("type_", &self.type_)
			.field("media_type", &self.media_type)
			.field("resolution", &self.resolution)
			.field("comment", &self.comment)
			.field("data", &format_args!("<{} BYTES>", self.data.len()))
			.finish()
	}
}
