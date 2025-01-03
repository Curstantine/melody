use {
	blake3::Hash,
	bonsaidb::core::{key::Key, schema::Collection},
	serde::{Deserialize, Serialize},
};

use crate::{
	database::views::cover::CoverByTypeAndHash,
	errors::{self, Result},
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Key)]
#[serde(rename_all = "snake_case")]
pub enum CoverType {
	Artist,
	Release,
	Track,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverMediaType {
	Png,
	Jpeg,
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "covers", views = [CoverByTypeAndHash])]
pub struct Cover {
	pub type_: CoverType,
	pub media_type: CoverMediaType,
	pub resolution: (u16, u16),
	pub comment: Option<String>,
	pub has_thumb: bool,
	// TODO: hash get returned as bytes
	pub hash: Hash,
}

impl CoverMediaType {
	pub fn as_extension(&self) -> &'static str {
		match self {
			Self::Png => "png",
			Self::Jpeg => "jpg",
		}
	}

	pub fn from_codec_id(value: rsmpeg::ffi::AVCodecID) -> Result<Self> {
		use rsmpeg::ffi::{AVCodecID_AV_CODEC_ID_MJPEG, AVCodecID_AV_CODEC_ID_PNG};

		#[allow(non_upper_case_globals)]
		let type_ = match value {
			AVCodecID_AV_CODEC_ID_MJPEG => Self::Jpeg,
			AVCodecID_AV_CODEC_ID_PNG => Self::Png,
			_ => return Err(errors::pre::unsupported_media_type(&value.to_string())),
		};

		Ok(type_)
	}
}
