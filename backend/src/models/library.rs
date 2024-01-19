use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
	pub name: String,
	pub scan_locations: Vec<String>,
}

impl Library {
	pub fn new(name: String, scan_locations: Vec<String>) -> Self {
		Self { name, scan_locations }
	}
}
