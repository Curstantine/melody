use std::{
	borrow::Cow,
	fmt::{Debug, Display},
};

use serde::Serialize;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Serialize, PartialEq)]
pub enum ErrorType {
	Io,
	Descriptive,
	Conversion,

	Tokio,
	Database,
	Tauri,
	Serde,
	Symphonia,
}

#[derive(Debug, Serialize)]
pub struct Error {
	pub type_: ErrorType,
	#[serde(borrow)]
	pub message: Cow<'static, str>,

	#[serde(borrow)]
	pub context: Option<Cow<'static, str>>,

	#[serde(skip)]
	pub source: Option<Box<dyn std::error::Error + Send>>,
}

impl Error {
	pub fn descriptive(message: &'static str) -> Self {
		Self {
			type_: ErrorType::Descriptive,
			message: Cow::Borrowed(message),
			context: None,
			source: None,
		}
	}

	pub fn conversion(message: &'static str, context: Option<Cow<'static, str>>) -> Self {
		Self {
			type_: ErrorType::Conversion,
			message: message.into(),
			context,
			source: None,
		}
	}

	pub fn with_context(mut self, context: Cow<'static, str>) -> Self {
		self.context = Some(context);
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

/// Convenience trait to implement From<T> for errors while including contextual data.
///
/// Implement this trait only where it makes sense.
pub trait FromErrorWithContext<T>: Sized {
	type Context;
	fn from_with_context(error: T, context: Self::Context) -> Self;
}

/// Convenience type for knowing what type of error std::io::Error is about.
#[derive(Debug, PartialEq)]
pub enum StdIoErrorType {
	/// Path to target
	File(Cow<'static, str>),
	Other,
}

impl FromErrorWithContext<std::io::Error> for Error {
	type Context = StdIoErrorType;

	fn from_with_context(error: std::io::Error, context: StdIoErrorType) -> Self {
		use std::io::ErrorKind as EK;

		let (message, context): (Cow<'static, str>, Cow<'static, str>) = match context {
			StdIoErrorType::File(x) => match error.kind() {
				EK::AlreadyExists => {
					let y = format!("Creation operation at {x} returned an error implying target file already exists.");
					(Cow::Borrowed("File already exists"), Cow::Owned(y))
				}
				EK::NotFound => {
					let y = format!("Read operation at {x} failed with not found.");
					(Cow::Borrowed("File not found"), Cow::Owned(y))
				}
				EK::UnexpectedEof => {
					let y = format!("Prematurely found end of file while reading {x}");
					(Cow::Borrowed("Unexpected end-of-file"), Cow::Owned(y))
				}
				_ => {
					let y = format!("Unhandled error '{}' at {x}", error.kind());
					(Cow::Borrowed("IO operation failure"), Cow::Owned(y))
				}
			},
			StdIoErrorType::Other => (Cow::Borrowed("IO operation failure"), Cow::Owned(error.to_string())),
		};

		Self {
			type_: ErrorType::Io,
			message,
			context: Some(context),
			source: Some(Box::new(error)),
		}
	}
}

impl FromErrorWithContext<std::num::ParseIntError> for Error {
	type Context = Cow<'static, str>;

	fn from_with_context(error: std::num::ParseIntError, context: Self::Context) -> Self {
		use std::num::IntErrorKind;

		let context: Cow<'static, str> = match error.kind() {
			IntErrorKind::InvalidDigit => Cow::Owned(format!("Given integer to parse was invalid, got: {context}")),
			IntErrorKind::Empty | IntErrorKind::Zero => Cow::Borrowed("Given value was empty"),
			IntErrorKind::NegOverflow | IntErrorKind::PosOverflow => {
				let x = format!("Failed to parse the integer to the given type, integer overflow!\nGot: {context}");
				Cow::Owned(x)
			}
			_ => panic!("Unknown error variant from std::num::ParseIntError: #{:#?}", error),
		};

		Self {
			type_: ErrorType::Conversion,
			message: Cow::Borrowed("Erroneous integer conversion"),
			context: Some(context),
			source: Some(Box::new(error)),
		}
	}
}

impl FromErrorWithContext<chrono::ParseError> for Error {
	type Context = Cow<'static, str>;

	fn from_with_context(error: chrono::ParseError, context: Self::Context) -> Self {
		use chrono::format::ParseErrorKind as PEK;

		let context: Cow<'static, str> = match error.kind() {
			PEK::BadFormat => Cow::Owned(format!("Date is badly formatted, got: {context}")),
			PEK::OutOfRange => Cow::Owned(format!("Date is out of spec range, got: {context}")),
			PEK::Impossible => Cow::Owned(format!("Date is impossible to parse, got: {context}")),
			PEK::NotEnough | PEK::Invalid | PEK::TooShort | PEK::TooLong => {
				Cow::Borrowed("Date is invalid, it's either not following the supported formats or is malformed")
			}
			_ => panic!("Unknown error variant from chrono::ParseError: {:#?}", error),
		};

		Self {
			type_: ErrorType::Conversion,
			message: Cow::Borrowed("Errornous date parse"),
			context: Some(context),
			source: Some(Box::new(error)),
		}
	}
}

impl From<tokio::task::JoinError> for Error {
	fn from(error: tokio::task::JoinError) -> Self {
		let context: Cow<'static, str> = if error.is_cancelled() {
			Cow::Borrowed("Task failed from unsafely cancelling it")
		} else {
			// We would want to panic if this error is neither a cancellation error or an inner panic anyway.
			let reason = error.into_panic();
			Cow::Owned(format!("Task most failed from a caller panic'ing, here: {:#?}", error))
		};

		Self {
			type_: ErrorType::Tokio,
			message: Cow::Borrowed("Tokio task failure"),
			context: Some(context),
			source: Some(Box::new(error)),
		}
	}
}

impl From<tauri::Error> for Error {
	fn from(error: tauri::Error) -> Self {
		use tauri::Error as TE;

		let context: Cow<'static, str> = match error {
			TE::Setup(x) => Cow::Owned(format!("Setup hook failed with: {x}")),
			TE::Io(x) => {
				let e = Error::from_with_context(x, StdIoErrorType::Other);
				Cow::Owned(format!("IO error with:\n{e}"))
			}
			TE::JoinError(x) => {
				let e = Error::from(x);
				let y = format!("Hmm, this shouldn't happen. Tauri met with a tokio task error: {e}");
				Cow::Owned(y)
			}
			_ => Cow::Owned(format!("Unhandled error {error}")),
		};

		Self {
			type_: ErrorType::Tauri,
			message: Cow::Borrowed("Tauri failure"),
			context: Some(context),
			source: Some(Box::new(error)),
		}
	}
}

impl From<bonsaidb::local::Error> for Error {
	fn from(error: bonsaidb::local::Error) -> Self {
		use bonsaidb::local::Error as BE;

		let (message, context): (&'static str, Cow<'static, str>) = match error {
			BE::Nebari(x) => {
				let y = format!("BonsaiDB (local) returned a Nebari error: {x}");
				("Database storage layer failure", Cow::Owned(y))
			}
			BE::Core(x) => {
				let y = format!("BonsaiDB (local) errored with: {x}", x = Error::from(x));
				("Database core failure", Cow::Owned(y))
			}
			BE::TaskJoin(x) => {
				let e = Error::from(x);
				let y = format!("Hmm, this shouldn't happen. BonsaiDB (local) returned a tokio task error: {e}");
				("Database threading failure", Cow::Owned(y))
			}
			BE::Io(x) => {
				let e = Error::from_with_context(x, StdIoErrorType::Other);
				let y = format!("BonsaiDB (local) returned an io error.\nMost probably while accessing the database files.\nReturned error: {e}");
				("Database io error", Cow::Owned(y))
			}
			_ => (
				"BonsaiDB (Local) returned an unhandled error",
				Cow::Owned(format!("{error}")),
			),
		};

		Self {
			type_: ErrorType::Database,
			message: Cow::Borrowed(message),
			context: Some(context),
			source: Some(Box::new(error)),
		}
	}
}

impl From<bonsaidb::core::Error> for Error {
	fn from(error: bonsaidb::core::Error) -> Self {
		use bonsaidb::core::Error as BE;

		let (message, context): (&'static str, Cow<'static, str>) = match error {
			BE::ViewNotFound => ("Database view not found", Cow::Owned(error.to_string())),
			BE::DatabaseNotFound(x) => {
				let y = format!("Database by name '{x}' doesn't exist");
				("Database not found", Cow::Owned(y))
			}
			_ => (
				"BonsaiDB (Core) returned an unhandled error",
				Cow::Owned(format!("{error}")),
			),
		};

		Self {
			type_: ErrorType::Database,
			message: Cow::Borrowed(message),
			context: Some(context),
			source: Some(Box::new(error)),
		}
	}
}

impl<T: Debug + Send + 'static> From<bonsaidb::core::schema::InsertError<T>> for Error {
	fn from(value: bonsaidb::core::schema::InsertError<T>) -> Self {
		let x = format!("Failed to insert: {:?}", value.contents);
		Error::from(value.error).with_context(Cow::Owned(x))
	}
}

impl From<serde_json::Error> for Error {
	fn from(error: serde_json::Error) -> Self {
		Self {
			type_: ErrorType::Serde,
			message: Cow::Borrowed("JSON Serialization error"),
			context: Some(Cow::Owned(format!("{:?}", error))),
			source: Some(Box::new(error)),
		}
	}
}

impl FromErrorWithContext<symphonia::core::errors::Error> for Error {
	type Context = Cow<'static, str>;

	fn from_with_context(error: symphonia::core::errors::Error, context: Self::Context) -> Self {
		use symphonia::core::errors::Error as SE;

		let (message, context): (&'static str, Cow<'static, str>) = match error {
			SE::DecodeError(_) => {
				let y = format!("The stream is either malformed or could not be decoded.\nFile: {context}");
				("Decode error", Cow::Owned(y))
			}
			SE::Unsupported(x) => {
				let y = format!(
					"Symphonia was invoked with an unsupported codec/container feature {x} while reading {context}"
				);
				("Symphonia feature not supported", Cow::Owned(y))
			}
			SE::IoError(x) => {
				let e = Error::from_with_context(x, StdIoErrorType::File(context));
				("Symphonia io error", Cow::Owned(format!("{e}")))
			}
			_ => ("Symphonia returned an unhandled error", Cow::Owned(format!("{error}"))),
		};

		Self {
			type_: ErrorType::Symphonia,
			message: Cow::Borrowed(message),
			context: Some(context),
			source: Some(Box::new(error)),
		}
	}
}
