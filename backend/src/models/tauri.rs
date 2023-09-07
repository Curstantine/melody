use serde::Serialize;

#[derive(Serialize)]
pub struct WindowEvent<T, P>
where
	T: WindowEventType,
	P: Serialize + Clone,
{
	#[serde(rename = "type")]
	pub _type: T,
	pub payload: P,
}

impl<T: WindowEventType, P: Serialize + Clone> WindowEvent<T, P> {
	pub fn new(_type: T, payload: P) -> Self {
		Self { _type, payload }
	}

	pub fn emit(self, window: &tauri::Window) -> crate::errors::Result<()> {
		let event_name = self._type.get_event_name();
		window.emit(event_name, self.payload)?;
		Ok(())
	}
}

pub trait WindowEventType {
	fn get_event_name(&self) -> &'static str;
}
