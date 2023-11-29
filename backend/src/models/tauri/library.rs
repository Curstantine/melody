use std::path::PathBuf;

use serde::Serialize;

use crate::errors::extra::CopyableSerializableError;

use super::{ActionPathedError, ActionPayload, WindowEventType};

pub type LibraryActionPayload = ActionPayload<LibraryAction, ActionPathedError>;
impl LibraryActionPayload {
	pub fn reading(data: LibraryActionData) -> Self {
		Self::Ok(LibraryAction::Reading(data))
	}

	pub fn indexing(data: LibraryActionData) -> Self {
		Self::Ok(LibraryAction::Indexing(data))
	}

	pub fn error(error: CopyableSerializableError, path: PathBuf) -> Self {
		Self::Error(ActionPathedError { error, path })
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryActionData {
	pub total: u64,
	pub current: u64,
	pub path: PathBuf,
}

impl LibraryActionData {
	pub fn new(total: u64, current: u64, path: PathBuf) -> Self {
		Self { total, current, path }
	}
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LibraryAction {
	Reading(LibraryActionData),
	Indexing(LibraryActionData),
}

#[derive(Debug, Serialize)]
pub enum LibraryEvent {
	Scan,
}

impl WindowEventType for LibraryEvent {
	fn get_event_name(&self) -> &'static str {
		match self {
			LibraryEvent::Scan => "library_scan",
		}
	}
}
