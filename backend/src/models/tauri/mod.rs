use std::{marker::PhantomData, path::PathBuf};

use serde::Serialize;

use crate::errors::{extra::CopyableSerializableError, Result};

pub mod library;
pub mod release;
pub mod resource;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum EventPayload<T: Serialize + Clone, E: Serialize + Clone> {
	Ok(T),
	Error(E),
}

#[derive(Debug, Clone, Serialize)]
pub struct SerializablePathedError {
	error: CopyableSerializableError,
	path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct Entity<T: Serialize> {
	pub id: u64,
	pub attributes: T,
}

impl<T: Serialize> Entity<T> {
	pub fn new(id: u64, attributes: T) -> Self {
		Self { id, attributes }
	}
}

pub struct WindowEventManager<T, D, E>
where
	T: WindowEventType,
	D: Serialize + Clone,
	E: Serialize + Clone,
{
	pub inner: T,
	data: PhantomData<D>,
	error: PhantomData<E>,
}

impl<T, D, E> WindowEventManager<T, D, E>
where
	T: WindowEventType,
	D: Serialize + Clone,
	E: Serialize + Clone,
{
	pub fn new(inner: T) -> Self {
		Self {
			inner,
			data: PhantomData,
			error: PhantomData,
		}
	}

	pub fn emit(&self, window: &tauri::Window, payload: EventPayload<D, E>) -> Result<()> {
		let event_name = self.inner.get_name();
		window.emit(event_name, payload)?;
		Ok(())
	}
}

pub trait WindowEventType: Sized {
	fn get_name(&self) -> &'static str;
}
