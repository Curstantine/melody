use std::path::PathBuf;

use serde::Serialize;

use crate::errors::{extra::CopyableSerializableError, Result};

pub mod library;
pub mod release;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ActionPayload<T: Serialize + Clone, E: Serialize + Clone> {
	Ok(T),
	Error(E),
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionPathedError {
	error: CopyableSerializableError,
	path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionEntity<T: Serialize> {
	pub id: u64,
	pub attributes: T,
}

impl<T: Serialize> ActionEntity<T> {
	pub fn new(id: u64, attributes: T) -> Self {
		Self { id, attributes }
	}
}

pub struct WindowEventManager<T: WindowEventType>(pub T);

impl<T: WindowEventType> WindowEventManager<T> {
	pub fn emit(&self, window: &tauri::Window, payload: impl Serialize + Clone) -> Result<()> {
		let event_name = self.0.get_event_name();
		window.emit(event_name, payload)?;
		Ok(())
	}
}

pub trait WindowEventType: Sized {
	fn get_event_name(&self) -> &'static str;
}
