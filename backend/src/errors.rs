use serde::Serialize;
use std::borrow::Cow;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
	Io,
	Conversion,
	Database,
	Encoder,
	Other,
}

#[derive(Debug, Clone, Serialize)]
pub struct Error {
	#[serde(skip)]
	pub kind: ErrorKind,

	#[serde(borrow)]
	pub short: Cow<'static, str>,

	#[serde(borrow)]
	pub message: Option<Cow<'static, str>>,
}

impl Error {
	pub fn new(short: &'static str, message: Cow<'static, str>) -> Self {
		Self {
			kind: ErrorKind::Other,
			short: Cow::Borrowed(short),
			message: Some(message),
		}
	}

	/// Appends a string to the message field.
	///
	/// If the message field is non empty, the string will appended to the field along with a newline.
	/// For cases where message is null, the string is clone to give the Cow ownership.
	pub fn append_message(mut self, message: &str) -> Self {
		if let Some(val) = &mut self.message {
			let x = val.to_mut();

			if !x.is_empty() {
				x.push('\n');
			}

			x.push_str(message);
		} else {
			self.message = Some(Cow::Owned(message.to_string()))
		}

		self
	}
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.short)?;

		if let Some(message) = &self.message {
			write!(f, " [Message: {}]", message)?;
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
		}
	}
}

impl From<std::num::ParseFloatError> for Error {
	fn from(value: std::num::ParseFloatError) -> Self {
		Self {
			kind: ErrorKind::Conversion,
			short: Cow::Borrowed("Conversion: float conversion error"),
			message: Some(Cow::Owned(value.to_string())),
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
		}
	}
}

impl<T: std::fmt::Debug + Send + 'static> From<bonsaidb::core::schema::InsertError<T>> for Error {
	fn from(value: bonsaidb::core::schema::InsertError<T>) -> Self {
		let x = format!("Failed to insert: {:?}", value.contents);

		Self {
			kind: ErrorKind::Database,
			short: Cow::Borrowed("BonsaiDB: Insert failure"),
			message: Some(Cow::Owned(x)),
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
		}
	}
}

impl From<serde_json::Error> for Error {
	fn from(value: serde_json::Error) -> Self {
		Self {
			kind: ErrorKind::Other,
			short: Cow::Borrowed("Serde: JSON Serialization error"),
			message: Some(Cow::Owned(value.to_string())),
		}
	}
}

pub mod pre {
	use std::borrow::Cow;

	use crate::errors::{Error, ErrorKind};

	#[inline]
	pub fn probe_no_meta() -> Error {
		Error {
			kind: ErrorKind::Encoder,
			short: Cow::Borrowed("Probe: No metadata"),
			message: Some(Cow::Borrowed("Couldn't find metadata in the track")),
		}
	}

	#[inline]
	pub fn probe_no_tags() -> Error {
		Error {
			kind: ErrorKind::Encoder,
			short: Cow::Borrowed("Probe: No tags"),
			message: Some(Cow::Borrowed("Couldn't find tags related to the track while probing")),
		}
	}

	#[inline]
	pub fn unsupported_media_type(type_: &str) -> Error {
		let message = format!("Unsupported media type '{type_}' was passed.");

		Error {
			kind: ErrorKind::Other,
			short: Cow::Borrowed("Invalid media type"),
			message: Some(Cow::Owned(message)),
		}
	}

	#[inline]
	pub fn unsupported_image_type(ext: &str) -> Error {
		let message = format!("Unsupported image file extension type: '{ext}'");

		Error {
			kind: ErrorKind::Other,
			short: Cow::Borrowed("Invalid image type"),
			message: Some(Cow::Owned(message)),
		}
	}
}
