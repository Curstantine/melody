use std::path::PathBuf;

use serde::Serialize;

use crate::{
	errors::Error,
	models::tauri::{EventPayload, SerializablePathedError, WindowEventManager, WindowEventType},
};

pub type LibraryEventManager = WindowEventManager<LibraryEventType, LibraryEvent, SerializablePathedError>;

pub type LibraryEventPayload = EventPayload<LibraryEvent, SerializablePathedError>;
impl LibraryEventPayload {
	pub fn indexing(data: LibraryEventData) -> Self {
		Self::Ok(LibraryEvent::Indexing(data))
	}

	pub fn scanning(data: PathBuf) -> Self {
		Self::Ok(LibraryEvent::Scanning(data))
	}

	pub fn error(error: Error, path: PathBuf) -> Self {
		Self::Error(SerializablePathedError { error, path })
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryEventData {
	pub total: u64,
	pub current: u64,
	pub path: PathBuf,
}

impl LibraryEventData {
	pub fn new(total: u64, current: u64, path: PathBuf) -> Self {
		Self { total, current, path }
	}
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum LibraryEvent {
	Scanning(PathBuf),
	Indexing(LibraryEventData),
}

#[derive(Debug)]
pub enum LibraryEventType {
	Scan,
}

impl WindowEventType for LibraryEventType {
	fn get_name(&self) -> &'static str {
		match self {
			LibraryEventType::Scan => "scan",
		}
	}
}
