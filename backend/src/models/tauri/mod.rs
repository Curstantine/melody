use serde::Serialize;

use crate::errors::Result;

pub mod library;

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
