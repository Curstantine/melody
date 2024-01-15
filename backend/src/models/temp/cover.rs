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
	/// Implies whether this cover needs a thumb.
	///
	/// True for all images where [TempCover::resolution] is >512
	pub fn needs_thumb(&self) -> bool {
		self.resolution.0 > 512 || self.resolution.1 > 512
	}

	pub fn into_cover(self, hash: Hash, has_thumb: bool) -> Cover {
		Cover {
			type_: self.type_,
			media_type: self.media_type,
			resolution: self.resolution,
			comment: self.comment,
			hash,
			has_thumb,
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
