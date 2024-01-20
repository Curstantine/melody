use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
	pub theme: Option<Theme>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Theme {
	pub name: String,
}
