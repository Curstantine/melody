use std::collections::HashMap;

use serde::Serialize;

use crate::database::models::{person::Person, release::Release, resource::Resource};

use super::Entity;

pub type ReleaseEntity = Entity<Release>;

#[derive(Debug, Serialize)]
pub struct DisplayReleases {
	pub releases: HashMap<u64, Release>,
	pub artists: HashMap<u64, Person>,
	pub covers: HashMap<u64, Resource>,
}
