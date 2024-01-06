use blake3::Hash;
use bonsaidb::core::{key::Key, schema::Collection};
use serde::{Deserialize, Serialize};

use crate::{
	database::views::resource::ResourceByTypeAndHash,
	errors::{self},
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Key)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
	Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Key)]
#[serde(rename_all = "snake_case")]
pub enum ResourceRelationType {
	Artist,
	Release,
	Track,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceMediaType {
	Png,
	Jpeg,
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "resources", views = [ResourceByTypeAndHash])]
pub struct Resource {
	pub type_: ResourceType,
	pub relation_type: ResourceRelationType,
	pub media_type: ResourceMediaType,
	pub has_thumb: bool,
	pub hash: Hash,
}

impl ResourceMediaType {
	pub fn to_extension(&self) -> &'static str {
		match self {
			Self::Png => "png",
			Self::Jpeg => "jpg",
		}
	}

	pub fn from_ffmpeg(value: u32) -> errors::Result<Self> {
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
