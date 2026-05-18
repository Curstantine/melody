use anyhow::{Result, anyhow};
use blake3::Hash;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverType {
	Artist,
	Release,
	Track,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverMediaType {
	Png,
	Jpeg,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cover {
	pub id: Uuid,
	pub type_: CoverType,
	pub media_type: CoverMediaType,
	pub resolution: (u16, u16),
	pub comment: Option<String>,
	pub has_thumb: bool,
	pub hash: Hash,
}

impl Cover {
	pub fn temp(
		type_: CoverType,
		media_type: CoverMediaType,
		resolution: (u16, u16),
		comment: Option<String>,
		has_thumb: bool,
		hash: Hash,
	) -> Self {
		Self {
			id: Uuid::nil(),
			type_,
			media_type,
			resolution,
			comment,
			has_thumb,
			hash,
		}
	}

	pub fn as_new(&mut self) -> &Self {
		self.id = Uuid::now_v7();
		self
	}
}

impl CoverMediaType {
	pub fn as_extension(&self) -> &'static str {
		match self {
			Self::Png => "png",
			Self::Jpeg => "jpg",
		}
	}

	pub fn from_codec_id(value: rsmpeg::avcodec::AVCodecID) -> Result<Self> {
		use rsmpeg::ffi::{AV_CODEC_ID_MJPEG, AV_CODEC_ID_PNG};

		#[allow(non_upper_case_globals)]
		let type_ = match value {
			AV_CODEC_ID_MJPEG => Self::Jpeg,
			AV_CODEC_ID_PNG => Self::Png,
			_ => return Err(anyhow!("Unknown AVCodecID: {}", value)),
		};

		Ok(type_)
	}
}
