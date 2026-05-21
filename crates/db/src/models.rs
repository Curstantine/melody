use serde::{Deserialize, Serialize};

pub mod cover;
pub mod label;
pub mod person;
pub mod release;
pub mod tag;
pub mod temp;
pub mod track;

#[derive(Debug, Serialize, Deserialize)]
pub enum CountryCode {
	Worldwide,
	Japan,
	Other(String),
}

impl FromTag for CountryCode {
	fn from_tag(value: &str) -> Self {
		match value.to_lowercase().as_str() {
			"xw" => Self::Worldwide,
			"jp" => Self::Japan,
			_ => Self::Other(value.to_owned()),
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ScriptCode {
	Latin,
	Japanese,
	Other(String),
}

impl FromTag for ScriptCode {
	fn from_tag(value: &str) -> Self {
		match value.to_lowercase().as_str() {
			"latn" => Self::Latin,
			"jpan" => Self::Japanese,
			_ => Self::Other(value.to_owned()),
		}
	}
}

pub trait FromTag: Sized {
	fn from_tag(value: &str) -> Self;
}
