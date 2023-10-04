use bonsaidb::core::schema::Collection;
use serde::{Deserialize, Serialize};

use crate::database::views::library::LibraryByName;

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "libraries", views = [LibraryByName])]
pub struct Library {
	pub name: String,
	pub scan_locations: Vec<String>,
}

impl Library {
	pub fn new(name: String, scan_locations: Vec<String>) -> Self {
		Self { name, scan_locations }
	}
}
