use bonsaidb::core::schema::Schema;
use serde::{Deserialize, Serialize};

pub mod label;
pub mod library;
pub mod person;
pub mod release;
pub mod tag;
pub mod track;

#[derive(Debug, Schema)]
#[schema(name = "default", collections = [
    label::Label,
    library::Library,
    person::Person,
    release::Release,
    tag::Tag,
    track::Track,
])]
pub struct LocalSchema;

#[derive(Debug, Serialize, Deserialize)]
pub enum CountryCode {
	Worldwide,
	Other(String),
}

impl FromTag for CountryCode {
	type Error = std::convert::Infallible;

	fn from_tag(value: &str) -> Result<Self, Self::Error> {
		let value = match value.to_lowercase().as_str() {
			"xw" => Self::Worldwide,
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
	pub id: String,
	pub name: String,
	pub join: Option<String>,
}

pub trait FromTag: Sized {
	type Error;
	fn from_tag(value: &str) -> Result<Self, Self::Error>;
}
