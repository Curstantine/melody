use std::collections::HashMap;

use serde::Serialize;

use crate::database::models::{person::Person, track::Track};

#[derive(Debug, Serialize)]
pub struct DisplayTrackList {
	pub tracks: Vec<Track>,
	pub artists: HashMap<u64, Person>,
}
