use serde::Serialize;
use std::{borrow::Cow, fmt, path::Path};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
	Io,
	Conversion,
	Database,
	Other,
}

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
	#[serde(skip)]
	pub kind: ErrorKind,

	#[serde(borrow)]
	pub short: Cow<'static, str>,

	#[serde(borrow)]
	pub message: Option<Cow<'static, str>>,

	pub data: Option<ErrorData>,
}

impl Error {
	pub fn new(short: &'static str, message: Option<&'static str>) -> Self {
		Self {
			kind: ErrorKind::Other,
			short: Cow::Borrowed(short),
			message: message.map(Cow::Borrowed),
			data: None,
		}
	}

	pub fn set_context(mut self, context: Cow<'static, str>) -> Self {
		self.message = Some(context);
		self
	}

	pub fn set_data(mut self, data: ErrorData) -> Self {
		self.data = Some(data);
		self
	}

	pub fn set_path_data(mut self, data: &Path) -> Self {
		self.set_data(ErrorData::Path(Cow::Borrowed(data)))
	}

	pub fn set_str_data(mut self, data: &'_ str) -> Self {
		self.set_data(ErrorData::String(Cow::Borrowed(data)))
	}
}

impl std::fmt::Display for ErrorData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ErrorData::Path(path) => write!(f, "{}", path.to_string_lossy()),
			ErrorData::String(string) => write!(f, "{}", string),
		}
	}
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.short)?;

		if let Some(message) = &self.message {
			write!(f, " [Message: {}]", message)?;
		}

		if let Some(data) = &self.data {
			write!(f, " [Data: {}]", data)?;
		}

		Ok(())
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		None
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
			kind: ErrorKind::Io,
			short: Cow::Borrowed(short),
			message: Some(message),
			data: None,
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
			kind: ErrorKind::Conversion,
			short: Cow::Borrowed("Conversion: integer conversion error"),
			message: Some(Cow::Borrowed(message)),
			data: None,
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
			kind: ErrorKind::Conversion,
			short: Cow::Borrowed("Chrono: failed to parse date"),
			message: Some(Cow::Borrowed(message)),
			data: None,
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
			kind: ErrorKind::Other,
			short: Cow::Borrowed("Tokio: Task join failure"),
			message: Some(message),
			data: None,
		}
	}
}

impl From<tauri::Error> for Error {
	fn from(value: tauri::Error) -> Self {
		use tauri::Error as TE;

		let (short, message): (&'static str, Cow<'static, str>) = match value {
			TE::Setup(x) => (
				"Tauri: Setup hook failed",
				Cow::Owned(format!("Setup hook failed with: {:?}", x)),
			),
			TE::Io(x) => {
				let e = Error::from(x);
				("Tauri: IO Error", Cow::Owned(e.to_string()))
			}
			TE::JoinError(x) => {
				let e = Error::from(x);
				let y = format!("Hmm, this shouldn't happen. Tauri met with a tokio task error: {e}");
				("Tauri: Tokio task error", Cow::Owned(y))
			}
			_ => ("Tauri: Unhandled error", Cow::Owned(value.to_string())),
		};

		Self {
			kind: ErrorKind::Other,
			short: Cow::Borrowed(short),
			message: Some(message),
			data: None,
		}
	}
}

impl From<bonsaidb::core::Error> for Error {
	fn from(value: bonsaidb::core::Error) -> Self {
		use bonsaidb::core::Error as BE;

		let (short, message): (&'static str, Cow<'static, str>) = match value {
			BE::ViewNotFound => ("BonsaiDB: View not found", Cow::Owned(value.to_string())),
			BE::DatabaseNotFound(x) => (
				"BonsaiDB: Database not found",
				Cow::Owned(format!("Couldn't find a database under {x}")),
			),
			_ => ("BonsaiDB: Unhandled error", Cow::Owned(value.to_string())),
		};

		Self {
			kind: ErrorKind::Database,
			short: Cow::Borrowed(short),
			message: Some(message),
			data: None,
		}
	}
}

impl From<bonsaidb::local::Error> for Error {
	fn from(value: bonsaidb::local::Error) -> Self {
		use bonsaidb::local::Error as BE;

		let (short, message): (&'static str, Cow<'static, str>) = match value {
			BE::Nebari(x) => (
				"BonsaiDB: Nebari failure",
				Cow::Owned(format!("Caught a storage layer error: {x}")),
			),
			BE::Core(x) => {
				let e = Error::from(x);
				("BonsaiDB: Core failure", Cow::Owned(e.to_string()))
			}
			BE::TaskJoin(x) => {
				let e = Error::from(x);
				let y = format!("Hmm, this shouldn't happen. BonsaiDB met with a tokio task error: {e}");
				("BonsaiDB: Tokio task error", Cow::Owned(y))
			}
			BE::Io(x) => {
				let e = Error::from(x);
				("BonsaiDB: IO error", Cow::Owned(e.to_string()))
			}
			_ => ("BonsaiDB: Unhandled error", Cow::Owned(value.to_string())),
		};

		Self {
			kind: ErrorKind::Database,
			short: Cow::Borrowed(short),
			message: Some(message),
			data: None,
		}
	}
}

impl<T: fmt::Debug + Send + 'static> From<bonsaidb::core::schema::InsertError<T>> for Error {
	fn from(value: bonsaidb::core::schema::InsertError<T>) -> Self {
		let x = format!("Failed to insert: {:?}", value.contents);

		Self {
			kind: ErrorKind::Database,
			short: Cow::Borrowed("BonsaiDB: Insert failure"),
			message: Some(Cow::Owned(x)),
			data: None,
		}
	}
}

impl From<symphonia::core::errors::Error> for Error {
	fn from(value: symphonia::core::errors::Error) -> Self {
		use symphonia::core::errors::Error as SE;

		let (short, message): (&'static str, Cow<'static, str>) = match value {
			SE::DecodeError(x) => (
				"Symphonia: Decode failure",
				Cow::Owned(format!("The stream is either malformed or could not be decoded. {x}")),
			),
			SE::Unsupported(x) => {
				let y = format!("Symphonia was invoked with an unsupported codec/container feature: {x}");
				("Symphonia: Unsupported feature", Cow::Owned(y))
			}
			SE::IoError(x) => {
				let e = Error::from(x);
				("Symphonia: IO error", Cow::Owned(e.to_string()))
			}
			_ => ("Symphonia: Unhandled error", Cow::Owned(value.to_string())),
		};

		Self {
			kind: ErrorKind::Database,
			short: Cow::Borrowed(short),
			message: Some(message),
			data: None,
		}
	}
}

impl From<image::ImageError> for Error {
	fn from(value: image::ImageError) -> Self {
		use image::ImageError as IE;

		let (short, message): (&'static str, Cow<'static, str>) = match value {
			IE::Encoding(e) => ("Image: Encode failure", Cow::Owned(e.to_string())),
			IE::Decoding(e) => ("Image: Decode failure", Cow::Owned(e.to_string())),
			IE::IoError(x) => {
				let e = Error::from(x);
				("Image: IO error", Cow::Owned(e.to_string()))
			}
			_ => ("Image: Unhandled error", Cow::Owned(value.to_string())),
		};

		Self {
			kind: ErrorKind::Other,
			short: Cow::Borrowed(short),
			message: Some(message),
			data: None,
		}
	}
}

impl From<serde_json::Error> for Error {
	fn from(value: serde_json::Error) -> Self {
		Self {
			kind: ErrorKind::Other,
			short: Cow::Borrowed("Serde: JSON Serialization error"),
			message: Some(Cow::Owned(value.to_string())),
			data: None,
		}
	}
}

pub mod pre {
	use super::Error;

	#[inline]
	pub fn symphonia_no_meta() -> Error {
		Error::new(
			"Symphonia: No metadata",
			Some("Couldn't find any metadata in track while probing"),
		)
	}

	#[inline]
	pub fn symphonia_no_tags() -> Error {
		Error::new(
			"Symphonia: No metadata",
			Some("Couldn't find tags related to the track while probing"),
		)
	}
}
