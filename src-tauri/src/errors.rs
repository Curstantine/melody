use std::fmt::{Display, Formatter};

use serde::{ser::SerializeStruct, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	Io(std::io::Error, Option<String>),
	TokioTask(tokio::task::JoinError),
	Tauri(tauri::Error),
	Descriptive(String),
	Database(polodb_core::Error),
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
			Self::Io(error, context) => ("io", error.to_string(), context.clone()),
			Self::TokioTask(error) => ("tokio_task", error.to_string(), None),
			Self::Tauri(error) => ("tauri", error.to_string(), None),
			Self::Descriptive(message) => ("descriptive", message.clone(), None),
			Self::Database(error) => ("database", error.to_string(), None),
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

impl From<polodb_core::Error> for Error {
	fn from(error: polodb_core::Error) -> Self {
		Self::Database(error)
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Io(error, _) => Some(error),
			Self::TokioTask(error) => Some(error),
			Self::Tauri(error) => Some(error),
			Self::Database(error) => Some(error),
			Self::Descriptive(_) => None,
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Descriptive(message) => write!(f, "{}", message),
			Self::Database(error) => write!(f, "Database error: {}", error),
			Self::TokioTask(error) => write!(f, "Tokio task error: {}", error),
			Self::Tauri(error) => write!(f, "Tauri error: {}", error),
			Self::Io(source, context) => {
				if let Some(context) = context {
					write!(f, "IO error: {}\nContext: {}", source, context)
				} else {
					write!(f, "IO error: {}", source)
				}
			}
		}
	}
}
