use std::borrow::Cow;

use bonsaidb::core::{key::Key, schema::Collection};
use serde::{Deserialize, Serialize};

use crate::{errors::Error, database::views::resource::ResourceByTypeAndHash};

use super::FromTag;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Key)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
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

impl FromTag for ResourceMediaType {
	type Error = Error;

	fn from_tag(value: &str) -> Result<Self, Self::Error> {
		let value = match value.to_lowercase().as_str() {
			"image/jpeg" | "image/jpg" => Self::Jpeg,
			"image/png" => Self::Png,
			_ => {
				let x = format!("Expected known resource media type, but got {}", value);
				return Err(Error::conversion("Unsupported media type", Some(Cow::Owned(x))));
			}
		};

		Ok(value)
	}
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "resources", views = [ResourceByTypeAndHash])]
pub struct Resource {
	pub type_: ResourceType,
	pub media_type: ResourceMediaType,
	pub path: String,
	pub hash: String,
}
