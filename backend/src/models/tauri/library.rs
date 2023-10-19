use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::errors::extra::CopyableSerializableError;

use super::WindowEventType;

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize)]
pub enum LibraryActionType {
	Reading,
	Indexing,
}

#[derive(Debug, Clone, Serialize)]
pub enum LibraryActionPayload {
	Ok(LibraryActionData),
	Error {
		error: CopyableSerializableError,
		path: PathBuf,
	},
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryActionData {
	#[serde(rename = "type")]
	pub type_: LibraryActionType,
	pub total: u64,
	pub current: u64,
	pub path: PathBuf,
}

impl LibraryActionData {
	pub fn reading(total: u64, current: u64, path: PathBuf) -> Self {
		Self {
			type_: LibraryActionType::Reading,
			total,
			current,
			path,
		}
	}

	pub fn indexing(total: u64, current: u64, path: PathBuf) -> Self {
		Self {
			type_: LibraryActionType::Indexing,
			total,
			current,
			path,
		}
	}
}
