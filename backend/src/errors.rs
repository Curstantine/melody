use std::fmt::{Display, Formatter};

use serde::{ser::SerializeStruct, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
enum BonsaiError {
	Local(bonsaidb::local::Error),
	Core(bonsaidb::core::Error),
}

#[derive(Debug)]
pub enum Error {
	Io(std::io::Error, Option<String>),
	TokioTask(tokio::task::JoinError),
	Tauri(tauri::Error),
	Descriptive(String),
	Bonsai(BonsaiError),
	Serde(serde_json::Error),
	ParseInt(std::num::ParseIntError),
	ChronoParse(chrono::ParseError),
}

impl Error {
	pub fn descriptive(message: impl Into<String>) -> Self {
		Self::Descriptive(message.into())
	}
}

impl Serialize for Error {
	/// Serializes this error to an externally consumable format.
	///
	/// E.g.: `{ "type": "io", "message": "...", context: null }`
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
		let mut state = serializer.serialize_struct("Error", 3)?;
		let (error_type, message, context) = match self {
			Self::Io(error, context) => ("io", error.to_string(), context.to_owned()),
			Self::TokioTask(error) => ("tokio_task", error.to_string(), None),
			Self::Tauri(error) => ("tauri", error.to_string(), None),
			Self::Descriptive(message) => ("descriptive", message.to_string(), None),
			Self::Serde(error) => ("serde", error.to_string(), None),
			Self::ParseInt(error) => ("parse_int", error.to_string(), None),
			Self::ChronoParse(error) => ("chrono_parse", error.to_string(), None),
			Self::Bonsai(inner) => match inner {
				BonsaiError::Local(error) => ("bonsai_local", error.to_string(), None),
				BonsaiError::Core(error) => ("bonsai_core", error.to_string(), None),
			},
		};

		state.serialize_field("type", error_type)?;
		state.serialize_field("message", &message)?;
		state.serialize_field("context", &context)?;
		state.end()
	}
}

impl From<std::io::Error> for Error {
	fn from(error: std::io::Error) -> Self {
		Self::Io(error, None)
	}
}

impl From<tokio::task::JoinError> for Error {
	fn from(error: tokio::task::JoinError) -> Self {
		Self::TokioTask(error)
	}
}

impl From<tauri::Error> for Error {
	fn from(error: tauri::Error) -> Self {
		Self::Tauri(error)
	}
}

impl From<bonsaidb::local::Error> for Error {
	fn from(error: bonsaidb::local::Error) -> Self {
		Self::Bonsai(BonsaiError::Local(error))
	}
}

impl From<bonsaidb::core::Error> for Error {
	fn from(error: bonsaidb::core::Error) -> Self {
		Self::Bonsai(BonsaiError::Core(error))
	}
}

impl<T> From<bonsaidb::core::schema::InsertError<T>> for Error {
	fn from(error: bonsaidb::core::schema::InsertError<T>) -> Self {
		Self::Bonsai(BonsaiError::Core(error.error))
	}
}

impl From<serde_json::Error> for Error {
	fn from(error: serde_json::Error) -> Self {
		Self::Serde(error)
	}
}

impl From<std::num::ParseIntError> for Error {
	fn from(error: std::num::ParseIntError) -> Self {
		Self::ParseInt(error)
	}
}

impl From<chrono::ParseError> for Error {
	fn from(error: chrono::ParseError) -> Self {
		Self::ChronoParse(error)
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Io(error, _) => Some(error),
			Self::TokioTask(error) => Some(error),
			Self::Tauri(error) => Some(error),
			Self::Serde(error) => Some(error),
			Self::ParseInt(error) => Some(error),
			Self::ChronoParse(error) => Some(error),
			Self::Descriptive(_) => None,
			Self::Bonsai(inner) => match inner {
				BonsaiError::Local(error) => Some(error),
				BonsaiError::Core(error) => Some(error),
			},
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Descriptive(message) => write!(f, "{}", message),
			Self::TokioTask(error) => write!(f, "Tokio task error: {}", error),
			Self::Tauri(error) => write!(f, "Tauri error: {}", error),
			Self::Serde(error) => write!(f, "Serde error: {}", error),
			Self::ParseInt(error) => write!(f, "ParseInt error: {}", error),
			Self::ChronoParse(error) => write!(f, "ChronoParse error: {}", error),
			Self::Io(source, context) => {
				if let Some(context) = context {
					write!(f, "IO error: {}\nContext: {}", source, context)
				} else {
					write!(f, "IO error: {}", source)
				}
			}
			Self::Bonsai(inner) => match inner {
				BonsaiError::Local(error) => write!(f, "Bonsai local error: {}", error),
				BonsaiError::Core(error) => write!(f, "Bonsai core error: {}", error),
			},
		}
	}
}
