use bonsaidb::core::schema::Collection;
use serde::{Deserialize, Serialize};

use crate::database::views::label::LabelByName;

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "labels", views = [LabelByName])]
pub struct Label {
	pub name: String,
	pub library_ids: Vec<u32>,
}
