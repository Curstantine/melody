use std::{
	borrow::Cow,
	fmt::{Debug, Display},
	path::Path,
};

use serde::Serialize;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum ErrorType {
	Io,
	Descriptive,
	Conversion,

	Tokio,
	Database,
	Tauri,
	Serde,
	Symphonia,
	Image,
}

#[derive(Debug, Serialize)]
/// Describes an app-wide error.
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
			message: Cow::Borrowed(message),
			context,
			source: None,
		}
	}

	#[inline]
	pub fn from_std_path(error: std::io::Error, path: &std::path::Path) -> Self {
		Self::from_with_ctx(error, IoErrorType::Path(path))
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

/// Convenience trait to implement contextual data for an error.
///
/// Implement this trait for all the error where a context message is expected.
pub trait ErrorContextData<T> {
	type ContextData<'a>;
	fn get_message(error: &T, data: Self::ContextData<'_>) -> (Cow<'static, str>, Option<Cow<'static, str>>);
}

/// Convenience trait to implement From<T> for errors while including contextual data.
///
/// Implement this trait only where it makes sense.
pub trait FromErrorWithContextData<T>: ErrorContextData<T> {
	fn from_with_ctx(error: T, data: Self::ContextData<'_>) -> Self;
}

/// Convenience type for knowing what type of error std::io::Error is about.
#[derive(Debug, PartialEq)]
pub enum IoErrorType<'a> {
	Path(&'a Path),
	Other,
}

impl ErrorContextData<std::io::Error> for Error {
	type ContextData<'a> = IoErrorType<'a>;

	fn get_message(error: &std::io::Error, data: IoErrorType<'_>) -> (Cow<'static, str>, Option<Cow<'static, str>>) {
		use std::io::ErrorKind as EK;

		let (message, context): (&'static str, Cow<'static, str>) = if let IoErrorType::Path(x) = data {
			let x = x.to_str().unwrap();
			match error.kind() {
				EK::AlreadyExists => {
					let y = format!("Create operation at {x} returned an error implying target file already exists.");
					("File already exists", Cow::Owned(y))
				}
				EK::NotFound => {
					let y = format!("Read operation at {x} failed with not found.");
					("File not found", Cow::Owned(y))
				}
				EK::UnexpectedEof => {
					let y = format!("Prematurely found end of file while reading {x}");
					("Unexpected end-of-file", Cow::Owned(y))
				}
				_ => {
					let y = format!("Unhandled error '{}' at {x}", error.kind());
					("IO operation failure", Cow::Owned(y))
				}
			}
		} else {
			("IO operation failure", Cow::Owned(error.to_string()))
		};

		(Cow::Borrowed(message), Some(context))
	}
}

impl FromErrorWithContextData<std::io::Error> for Error {
	fn from_with_ctx(error: std::io::Error, data: IoErrorType<'_>) -> Self {
		let (message, context) = Self::get_message(&error, data);

		Self {
			type_: ErrorType::Io,
			message,
			context,
			source: Some(Box::new(error)),
		}
	}
}

impl ErrorContextData<std::num::ParseIntError> for Error {
	type ContextData<'a> = Cow<'a, str>;

	fn get_message(
		error: &std::num::ParseIntError,
		data: Self::ContextData<'_>,
	) -> (Cow<'static, str>, Option<Cow<'static, str>>) {
		use std::num::IntErrorKind as IE;

		let context: Cow<'static, str> = match error.kind() {
			IE::InvalidDigit => Cow::Owned(format!("Given integer to parse was invalid, got: {data}")),
			IE::Empty | IE::Zero => Cow::Borrowed("Given value was empty"),
			IE::NegOverflow | IE::PosOverflow => {
				let x = format!("Failed to parse the integer to the given type, integer overflow!\nGot: {data}");
				Cow::Owned(x)
			}
			_ => panic!("Unknown error variant from std::num::ParseIntError: #{:#?}", error),
		};

		(Cow::Borrowed("Erroneous integer conversion"), Some(context))
	}
}

impl FromErrorWithContextData<std::num::ParseIntError> for Error {
	fn from_with_ctx(error: std::num::ParseIntError, data: Self::ContextData<'_>) -> Self {
		let (message, context) = Self::get_message(&error, data);

		Self {
			type_: ErrorType::Conversion,
			message,
			context,
			source: Some(Box::new(error)),
		}
	}
}

impl ErrorContextData<chrono::ParseError> for Error {
	type ContextData<'a> = Cow<'a, str>;

	fn get_message(
		error: &chrono::ParseError,
		data: Self::ContextData<'_>,
	) -> (Cow<'static, str>, Option<Cow<'static, str>>) {
		use chrono::format::ParseErrorKind as PEK;

		let context: Cow<'static, str> = match error.kind() {
			PEK::BadFormat => Cow::Owned(format!("Date is badly formatted, got: {data}")),
			PEK::OutOfRange => Cow::Owned(format!("Date is out of spec range, got: {data}")),
			PEK::Impossible => Cow::Owned(format!("Date is impossible to parse, got: {data}")),
			PEK::NotEnough | PEK::Invalid | PEK::TooShort | PEK::TooLong => {
				Cow::Borrowed("Date is invalid, it's either not following the supported formats or is malformed")
			}
			_ => panic!("Unknown error variant from chrono::ParseError: {:#?}", error),
		};

		(Cow::Borrowed("Errornous date parse"), Some(context))
	}
}

impl FromErrorWithContextData<chrono::ParseError> for Error {
	fn from_with_ctx(error: chrono::ParseError, data: Self::ContextData<'_>) -> Self {
		let (message, context) = Self::get_message(&error, data);

		Self {
			type_: ErrorType::Conversion,
			message,
			context,
			source: Some(Box::new(error)),
		}
	}
}

impl ErrorContextData<tokio::task::JoinError> for Error {
	type ContextData<'a> = Option<Cow<'a, str>>;

	fn get_message(
		error: &tokio::task::JoinError,
		_: Self::ContextData<'_>,
	) -> (Cow<'static, str>, Option<Cow<'static, str>>) {
		let context: Cow<'static, str> = if error.is_cancelled() {
			Cow::Borrowed("Task failed from unsafely cancelling it")
		} else {
			// We would want to panic if this error is neither a cancellation error or an inner panic anyway.
			Cow::Owned(format!("Task most failed from a caller panic'ing, here: {:#?}", error))
		};

		(Cow::Borrowed("Tokio task failure"), Some(context))
	}
}

impl From<tokio::task::JoinError> for Error {
	fn from(error: tokio::task::JoinError) -> Self {
		let (message, context) = Self::get_message(&error, None);

		Self {
			type_: ErrorType::Tokio,
			message,
			context,
			source: Some(Box::new(error)),
		}
	}
}

impl ErrorContextData<tauri::Error> for Error {
	type ContextData<'a> = Option<Cow<'a, str>>;

	fn get_message(error: &tauri::Error, _: Self::ContextData<'_>) -> (Cow<'static, str>, Option<Cow<'static, str>>) {
		use tauri::Error as TE;

		let context: Cow<'static, str> = match error {
			TE::Setup(x) => Cow::Owned(format!("Setup hook failed with: {x}")),
			TE::Io(x) => {
				let e = Error::get_message(x, IoErrorType::Other);
				Cow::Owned(format!("IO error with: {:?}", e))
			}
			TE::JoinError(x) => {
				let e = Error::get_message(x, None);
				let y = format!("Hmm, this shouldn't happen. Tauri met with a tokio task error: {:?}", e);
				Cow::Owned(y)
			}
			_ => Cow::Owned(format!("Unhandled error {error}")),
		};

		(Cow::Borrowed("Tauri failure"), Some(context))
	}
}

impl From<tauri::Error> for Error {
	fn from(error: tauri::Error) -> Self {
		let (message, context) = Self::get_message(&error, None);

		Self {
			type_: ErrorType::Tauri,
			message,
			context,
			source: Some(Box::new(error)),
		}
	}
}

impl ErrorContextData<bonsaidb::local::Error> for Error {
	type ContextData<'a> = Option<Cow<'a, str>>;

	fn get_message(
		error: &bonsaidb::local::Error,
		_: Self::ContextData<'_>,
	) -> (Cow<'static, str>, Option<Cow<'static, str>>) {
		use bonsaidb::local::Error as BE;

		let (message, context): (&'static str, Cow<'static, str>) = match &error {
			BE::Nebari(x) => {
				let y = format!("BonsaiDB (local) returned a Nebari error: {x}");
				("Database storage layer failure", Cow::Owned(y))
			}
			BE::Core(x) => {
				let y = format!("BonsaiDB (local) errored with: {:?}", Error::get_message(x, None));
				("Database core failure", Cow::Owned(y))
			}
			BE::TaskJoin(x) => {
				let e = Error::get_message(x, None);
				let y = format!(
					"Hmm, this shouldn't happen. BonsaiDB (local) returned a tokio task error: {:?}",
					e
				);
				("Database threading failure", Cow::Owned(y))
			}
			BE::Io(x) => {
				let e = Error::get_message(x, IoErrorType::Other);
				let y = format!("BonsaiDB (local) returned an io error.\nMost probably while accessing the database files.\nReturned error: {:?}", e);
				("Database io error", Cow::Owned(y))
			}
			_ => (
				"BonsaiDB (Local) returned an unhandled error",
				Cow::Owned(format!("{error}")),
			),
		};

		(Cow::Borrowed(message), Some(context))
	}
}

impl From<bonsaidb::local::Error> for Error {
	fn from(error: bonsaidb::local::Error) -> Self {
		let (message, context) = Self::get_message(&error, None);

		Self {
			type_: ErrorType::Database,
			message,
			context,
			source: Some(Box::new(error)),
		}
	}
}

impl ErrorContextData<bonsaidb::core::Error> for Error {
	type ContextData<'a> = Option<Cow<'a, str>>;

	fn get_message(
		error: &bonsaidb::core::Error,
		_: Self::ContextData<'_>,
	) -> (Cow<'static, str>, Option<Cow<'static, str>>) {
		use bonsaidb::core::Error as BE;

		let (message, context): (&'static str, Cow<'static, str>) = match &error {
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

		(Cow::Borrowed(message), Some(context))
	}
}

impl From<bonsaidb::core::Error> for Error {
	fn from(error: bonsaidb::core::Error) -> Self {
		let (message, context) = Self::get_message(&error, None);

		Self {
			type_: ErrorType::Database,
			message,
			context,
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

impl ErrorContextData<symphonia::core::errors::Error> for Error {
	type ContextData<'a> = &'a Path;

	fn get_message(
		error: &symphonia::core::errors::Error,
		data: Self::ContextData<'_>,
	) -> (Cow<'static, str>, Option<Cow<'static, str>>) {
		use symphonia::core::errors::Error as SE;

		let p = data.to_str().unwrap();
		let (message, context): (&'static str, Cow<'static, str>) = match &error {
			SE::DecodeError(_) => {
				let y = format!("The stream is either malformed or could not be decoded.\nFile: {p}");
				("Decode error", Cow::Owned(y))
			}
			SE::Unsupported(x) => {
				let y =
					format!("Symphonia was invoked with an unsupported codec/container feature {x} while reading {p}");
				("Symphonia feature not supported", Cow::Owned(y))
			}
			SE::IoError(x) => {
				let e = Error::get_message(x, IoErrorType::Path(data));
				("Symphonia io error", e.1.unwrap_or_default())
			}
			_ => ("Symphonia returned an unhandled error", Cow::Owned(format!("{error}"))),
		};

		(Cow::Borrowed(message), Some(context))
	}
}

impl FromErrorWithContextData<symphonia::core::errors::Error> for Error {
	fn from_with_ctx(error: symphonia::core::errors::Error, data: Self::ContextData<'_>) -> Self {
		let (message, context) = Self::get_message(&error, data);

		Self {
			type_: ErrorType::Symphonia,
			message,
			context,
			source: Some(Box::new(error)),
		}
	}
}

impl ErrorContextData<image::ImageError> for Error {
	type ContextData<'a> = &'a Path;

	fn get_message(
		error: &image::ImageError,
		data: Self::ContextData<'_>,
	) -> (Cow<'static, str>, Option<Cow<'static, str>>) {
		use image::ImageError as IE;

		let (message, context): (&'static str, Cow<'static, str>) = match &error {
			IE::Encoding(e) => {
				let context = format!("{}\nFile: {:?}", e, data);
				("Failed to encode image", Cow::Owned(context))
			}
			IE::Decoding(e) => {
				let context = format!("{}\nFile: {:?}", e, data);
				("Failed to decode image", Cow::Owned(context))
			}
			IE::IoError(e) => {
				let x = Error::get_message(e, IoErrorType::Path(data));
				("Image task failed with IO error", x.1.unwrap_or_default())
			}
			_ => ("Unhandled Image task error", Cow::Owned(format!("{error}"))),
		};

		(Cow::Borrowed(message), Some(context))
	}
}

impl FromErrorWithContextData<image::ImageError> for Error {
	fn from_with_ctx(error: image::ImageError, data: Self::ContextData<'_>) -> Self {
		let (message, context) = Self::get_message(&error, data);

		Self {
			type_: ErrorType::Image,
			context,
			message,
			source: Some(Box::new(error)),
		}
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

pub mod extra {
	use std::borrow::Cow;

	use serde::Serialize;

	use super::ErrorType;

	/// DO NOT use this struct for anything but cases where Serialize + Copy traits are required to be implemented.
	///
	/// Like in tauri's window events.
	#[derive(Debug, Clone, Serialize)]
	pub struct CopyableSerializableError {
		pub type_: ErrorType,
		#[serde(borrow)]
		pub message: Cow<'static, str>,

		#[serde(borrow)]
		pub context: Option<Cow<'static, str>>,
	}

	impl From<super::Error> for CopyableSerializableError {
		fn from(value: super::Error) -> Self {
			Self {
				type_: value.type_,
				message: value.message,
				context: value.context,
			}
		}
	}
}

pub mod new_error {
	use serde::Serialize;
	use std::{borrow::Cow, fmt, path::Path};

	#[derive(Debug, Serialize)]
	#[serde(tag = "type", content = "data", rename_all = "snake_case")]

	pub enum ErrorData {
		#[serde(borrow)]
		Path(Cow<'static, Path>),

		#[serde(borrow)]
		String(Cow<'static, str>),
	}

	#[derive(Debug, Serialize)]
	pub struct Error {
		#[serde(borrow)]
		pub short: Cow<'static, str>,

		#[serde(borrow)]
		pub message: Option<Cow<'static, str>>,

		pub data: Option<ErrorData>,

		#[serde(skip)]
		pub source: Option<Box<dyn std::error::Error + Send>>,
	}

	impl Error {
		pub fn new(short: &'static str, message: Option<&'static str>) -> Self {
			Self {
				short: Cow::Borrowed(short),
				message: message.map(Cow::Borrowed),
				data: None,
				source: None,
			}
		}

		pub fn set_context(mut self, context: Cow<'static, str>) -> Self {
			self.message.insert(context);
			self
		}

		pub fn set_data(mut self, data: ErrorData) -> Self {
			self.data.insert(data);
			self
		}
	}

	impl std::fmt::Display for Error {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "{}", self.short)?;

			if let Some(message) = &self.message {
				write!(f, " [Message: {}]", message)?;
			}

			if let Some(source) = &self.source {
				write!(f, " [Source: {}]", source)?;
			}

			write!(f, " [Data: {:?}]", &self.data)?;

			Ok(())
		}
	}

	impl std::error::Error for Error {
		fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
			self.source.as_ref().map(|boxed| boxed.as_ref() as _)
		}
	}

	// From implementations
	impl From<std::io::Error> for Error {
		fn from(value: std::io::Error) -> Self {
			use std::io::ErrorKind as EK;

			let (short, message): (&'static str, Cow<'static, str>) = match value.kind() {
				EK::AlreadyExists => (
					"IO: Already exists",
					Cow::Borrowed("Create operation returned an error implying target file already exists."),
				),
				EK::NotFound => ("IO: Not found", Cow::Borrowed("Read operation failed with not found.")),
				EK::UnexpectedEof => (
					"IO: Unexpected EOF",
					Cow::Borrowed("Found EOF prematurely while reading."),
				),
				_ => ("IO: Unhandled error", Cow::Owned(value.to_string())),
			};

			Self {
				short: Cow::Borrowed(short),
				message: Some(message),
				data: None,
				source: Some(Box::new(value)),
			}
		}
	}

	impl From<std::num::ParseIntError> for Error {
		fn from(value: std::num::ParseIntError) -> Self {
			use std::num::IntErrorKind as IE;

			let message: &'static str = match value.kind() {
				IE::InvalidDigit => "Given integer to parse was invalid",
				IE::Empty | IE::Zero => "Given value was empty",
				IE::NegOverflow | IE::PosOverflow => "Given value overflows the cast",
				_ => unreachable!(),
			};

			Self {
				short: Cow::Borrowed("Conversion: integer conversion error"),
				message: Some(Cow::Borrowed(message)),
				data: None,
				source: Some(Box::new(value)),
			}
		}
	}

	impl From<chrono::ParseError> for Error {
		fn from(value: chrono::ParseError) -> Self {
			use chrono::format::ParseErrorKind as PEK;

			let message: &'static str = match value.kind() {
				PEK::OutOfRange => "Date is out of spec range",
				PEK::Impossible => "Date is impossible to parse",
				PEK::BadFormat => "Date is badly formatted",
				PEK::NotEnough | PEK::Invalid | PEK::TooShort | PEK::TooLong => {
					"Date is invalid or not following the supported formats or is malformed"
				}
				_ => unreachable!(),
			};

			Self {
				short: Cow::Borrowed("Chrono: failed to parse date"),
				message: Some(Cow::Borrowed(message)),
				data: None,
				source: Some(Box::new(value)),
			}
		}
	}

	impl From<tokio::task::JoinError> for Error {
		fn from(value: tokio::task::JoinError) -> Self {
			let message: Cow<'static, str> = if value.is_cancelled() {
				Cow::Borrowed("Task failed from unsafely cancelling it")
			} else if let Ok(error) = value.try_into_panic() {
				Cow::Owned(format!("Task most failed from a caller panic'ing, here: {:#?}", error))
			} else {
				Cow::Borrowed("Task failed in an unexpected manner")
			};

			Self {
				short: Cow::Borrowed("Tokio: Task join failure"),
				message: Some(message),
				data: None,
				source: None,
			}
		}
	}

	impl<'a> From<tauri::Error> for Error {
		fn from(value: tauri::Error) -> Self {
			use tauri::Error as TE;

			let (short, message): (&'static str, Cow<'static, str>) = match &value {
				TE::Setup(x) => (
					"Tauri: Setup hook failed",
					Cow::Owned(format!("Setup hook failed with: {:?}", x)),
				),
				TE::Io(x) => {
					let e = Error::from();
					Cow::Owned(format!("IO error with: {:?}", e))
				}
				TE::JoinError(x) => {
					let e = Error::get_message(x, None);
					let y = format!("Hmm, this shouldn't happen. Tauri met with a tokio task error: {:?}", e);
					Cow::Owned(y)
				}
				_ => Cow::Owned(format!("Unhandled error {error}")),
			};

			Self {
				short: Cow::Borrowed(short),
				message: Some(message),
				data: (),
				source: value,
			}
		}
	}
}
