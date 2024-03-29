use bonsaidb::core::schema::Schema;
use serde::{Deserialize, Serialize};

use crate::constants::UNKNOWN_PERSON_ID;

pub mod cover;
pub mod label;
pub mod person;
pub mod release;
pub mod tag;
pub mod track;

#[derive(Debug, Schema)]
#[schema(name = "default", collections = [
    label::Label,
    person::Person,
    release::Release,
    tag::Tag,
    track::Track,
	cover::Cover,
])]
pub struct LocalSchema;

#[derive(Debug, Serialize, Deserialize)]
pub enum CountryCode {
	Worldwide,
	Japan,
	Other(String),
}

impl FromTag for CountryCode {
	type Error = std::convert::Infallible;

	fn from_tag(value: &str) -> Result<Self, Self::Error> {
		let value = match value.to_lowercase().as_str() {
			"xw" => Self::Worldwide,
			"jp" => Self::Japan,
			_ => Self::Other(value.to_owned()),
		};

		Ok(value)
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ScriptCode {
	Latin,
	Japanese,
	Other(String),
}

impl FromTag for ScriptCode {
	type Error = std::convert::Infallible;

	fn from_tag(value: &str) -> Result<Self, Self::Error> {
		let value = match value.to_lowercase().as_str() {
			"latn" => Self::Latin,
			"jpan" => Self::Japanese,
			_ => Self::Other(value.to_owned()),
		};

		Ok(value)
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InlinedArtist {
	pub id: u64,
	/// This is different from the [Person::name] field.
	///
	/// This field refers to an "alias" used by this [Person] in the context of the related entry.
	/// E.g. A person in release credit may use a different name than the one they use in the artist credit.
	pub credited_as: Option<String>,
	pub join: Option<String>,
}

impl InlinedArtist {
	/// Create an [InlinedArtist] that follows the default semantics for an unknown artist.
	pub fn unknown() -> Self {
		Self {
			id: UNKNOWN_PERSON_ID,
			credited_as: None,
			join: None,
		}
	}
}

pub trait FromTag: Sized {
	type Error;
	fn from_tag(value: &str) -> Result<Self, Self::Error>;
}
