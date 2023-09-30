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
	XW,
	Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ScriptCode {
	Latn,
	Jpan,
	Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InlinedArtist {
	pub id: String,
	pub name: String,
	pub join: Option<String>,
}
