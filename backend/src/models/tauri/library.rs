use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Serialize)]
pub struct LibraryGenericActionPayload {
	pub action_type: LibraryActionType,
	pub total: u32,
	pub current: u32,
	pub path: PathBuf,
}
