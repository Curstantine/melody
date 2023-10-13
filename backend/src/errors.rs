use std::fmt::{Debug, Display};

use serde::Serialize;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Serialize, PartialEq)]
pub enum ErrorType {
	IO,
	Descriptive,
	Conversion,

	Tokio,
	Database,
	Tauri,
	Serde,
}

#[derive(Debug, Serialize)]
pub struct Error {
	pub type_: ErrorType,
	pub message: String,
	pub context: Option<String>,

	#[serde(skip)]
	pub source: Option<Box<dyn std::error::Error + Send>>,
}

impl Error {
	pub fn descriptive(message: impl Into<String>) -> Self {
		Self {
			type_: ErrorType::Descriptive,
			message: message.into(),
			context: None,
			source: None,
		}
	}

	pub fn conversion(message: impl Into<String>, context: Option<impl Into<String>>) -> Self {
		Self {
			type_: ErrorType::Conversion,
			message: message.into(),
			context: context.map(|x| x.into()),
			source: None,
		}
	}

	pub fn with_context(mut self, context: impl Into<String>) -> Self {
		self.context = Some(context.into());
		self
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(context) = &self.context {
			write!(f, "{}: {}", self.message, context)
		} else {
			write!(f, "{}", self.message)
		}
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		self.source.as_ref().map(|boxed| boxed.as_ref() as _)
	}
}

impl From<std::io::Error> for Error {
	fn from(error: std::io::Error) -> Self {
		Self {
			type_: ErrorType::IO,
			message: error.to_string(),
			context: None,
			source: Some(Box::new(error)),
		}
	}
}

impl From<std::num::ParseIntError> for Error {
	fn from(error: std::num::ParseIntError) -> Self {
		use std::num::IntErrorKind;

		Self {
			type_: ErrorType::Conversion,
			message: match error.kind() {
				IntErrorKind::InvalidDigit => "Integer conversion failed due to an invalid digit".to_string(),
				_ => error.to_string(),
			},
			context: None,
			source: Some(Box::new(error)),
		}
	}
}

impl From<chrono::ParseError> for Error {
	fn from(error: chrono::ParseError) -> Self {
		Self {
			type_: ErrorType::Conversion,
			message: "Failed to parse date".to_string(),
			context: Some(error.to_string()),
			source: Some(Box::new(error)),
		}
	}
}

impl From<tokio::task::JoinError> for Error {
	fn from(error: tokio::task::JoinError) -> Self {
		Self {
			type_: ErrorType::Tokio,
			message: error.to_string(),
			context: None,
			source: Some(Box::new(error)),
		}
	}
}

impl From<tauri::Error> for Error {
	fn from(error: tauri::Error) -> Self {
		Self {
			type_: ErrorType::Tauri,
			message: error.to_string(),
			context: None,
			source: Some(Box::new(error)),
		}
	}
}

impl From<bonsaidb::local::Error> for Error {
	fn from(error: bonsaidb::local::Error) -> Self {
		Self {
			type_: ErrorType::Database,
			message: error.to_string(),
			context: None,
			source: Some(Box::new(error)),
		}
	}
}

impl From<bonsaidb::core::Error> for Error {
	fn from(error: bonsaidb::core::Error) -> Self {
		Self {
			type_: ErrorType::Database,
			message: error.to_string(),
			context: None,
			source: Some(Box::new(error)),
		}
	}
}

impl<T: Debug + Send + 'static> From<bonsaidb::core::schema::InsertError<T>> for Error {
	fn from(value: bonsaidb::core::schema::InsertError<T>) -> Self {
		Error::from(value.error).with_context(format!("Failed to insert: {:?}", value.contents))
	}
}

impl From<serde_json::Error> for Error {
	fn from(error: serde_json::Error) -> Self {
		Self {
			type_: ErrorType::Serde,
			message: error.to_string(),
			context: None,
			source: Some(Box::new(error)),
		}
	}
}
