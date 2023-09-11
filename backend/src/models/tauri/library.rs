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
			LibraryEvent::Scan => "library-scan",
		}
	}
}

#[derive(Clone, Debug, Serialize)]
pub struct LibraryScanEventPayload {
	pub total: u32,
	pub current: u32,
	pub path: PathBuf,
}