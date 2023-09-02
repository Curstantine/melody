use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	Io(std::io::Error, Option<String>),
	Descriptive(String),
	Database(polodb_core::Error),
}

impl From<std::io::Error> for Error {
	fn from(error: std::io::Error) -> Self {
		Self::Io(error, None)
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
            Self::Descriptive(_) => None,
            Self::Database(error) => Some(error),
            Self::Io(error, _) => Some(error),
        }
    }    
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Descriptive(message) => write!(f, "{}", message),
			Self::Database(error) => write!(f, "Database error: {}", error),
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
