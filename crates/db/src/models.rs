use serde::{Deserialize, Serialize};

use person::Person;

pub mod cover;
pub mod label;
pub mod person;
pub mod release;
pub mod tag;
pub mod track;

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


pub trait FromTag: Sized {
	type Error;
	fn from_tag(value: &str) -> Result<Self, Self::Error>;
}
