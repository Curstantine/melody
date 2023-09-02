use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Library {
	pub name: String,
	pub scan_locations: Vec<String>,
}
