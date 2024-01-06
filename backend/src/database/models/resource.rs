use std::borrow::Cow;

use blake3::Hash;
use bonsaidb::core::{key::Key, schema::Collection};
use rsmpeg::{avutil::AVMediaType, ffi::AVCodecID_AV_CODEC_ID_MJPEG};
use serde::{Deserialize, Serialize};

use crate::{
	database::views::resource::ResourceByTypeAndHash,
	errors::{self, Error},
};

use super::FromTag;

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
	/// Value corresponds to the AVCodecID
	#[cfg(feature = "ffmpeg")]
	pub fn from_ffmpeg(value: u32) -> errors::Result<Self> {
		let type_ = match value {
			AVCodecID_AV_CODEC_ID_MJPEG => Self::Jpeg,
			_ => return Err(errors::pre::unsupported_media_type(&value.to_string())),
		};

		Ok(type_)
	}

	pub fn to_extension(&self) -> &'static str {
		match self {
			Self::Png => "png",
			Self::Jpeg => "jpg",
		}
	}
}

impl FromTag for ResourceMediaType {
	type Error = Error;

	fn from_tag(value: &str) -> Result<Self, Self::Error> {
		let value = match value.to_lowercase().as_str() {
			"image/jpeg" | "image/jpg" => Self::Jpeg,
			"image/png" => Self::Png,
			_ => {
				let x = format!("Expected known resource media type, but got {}", value);
				return Err(Error::new_dyn("Unsupported media type", Cow::Owned(x)));
			}
		};

		Ok(value)
	}
}
